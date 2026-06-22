use ash::{config, env_probe, error, exec, llm, parser, risk, ui};
use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: ash <prompt...>");
        std::process::exit(1);
    }
    let prompt = args.join(" ");

    // Synchronous setup — load config and probe environment outside the async
    // runtime to avoid blocking the executor (WR-05).
    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(error::exit_code(&e));
        }
    };
    let env_summary = match env_probe::collect(
        cfg.collect_sys_info,
        cfg.collect_env_info,
        &cfg.tools_to_probe,
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(error::exit_code(&e));
        }
    };

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("ash: cannot create runtime: {e}");
            std::process::exit(1);
        }
    };
    let code = rt.block_on(run(&prompt, &cfg, &env_summary));
    std::process::exit(code);
}

async fn run(
    prompt: &str,
    cfg: &config::Config,
    env_summary: &env_probe::EnvSummary,
) -> i32 {
    let (draft, usage) = match llm::generate(prompt, env_summary, cfg).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return error::exit_code(&e);
        }
    };

    let parse_result = parser::parse(&draft.command);
    let audit = risk::audit(&parse_result, cfg);

    // Always print audit signals to stderr — transparency is part of the
    // security model. Never silently reject (WR-01).
    for s in &audit.signals {
        eprintln!("ash: {s}");
    }

    if audit.should_reject {
        // Show UI so the user sees what was blocked, but Execute is disabled.
        let ctx = ui::UiContext {
            tokens_in: usage.prompt_tokens.map(|v| v as i64),
            tokens_out: usage.completion_tokens.map(|v| v as i64),
            tokens_total: usage.total_tokens.map(|v| v as i64),
            risk_level: audit.risk_level,
        };
        if let Err(e) = ui::run_menu(
            &draft.command,
            draft.explanation.as_deref(),
            &ctx,
            true,
            true,
        ) {
            eprintln!("ash: UI error: {e}");
        }
        return 1;
    }

    let need_double = audit.need_double_confirm;
    let ctx = ui::UiContext {
        tokens_in: usage.prompt_tokens.map(|v| v as i64),
        tokens_out: usage.completion_tokens.map(|v| v as i64),
        tokens_total: usage.total_tokens.map(|v| v as i64),
        risk_level: audit.risk_level,
    };

    let result = match ui::run_menu(
        &draft.command,
        draft.explanation.as_deref(),
        &ctx,
        need_double,
        false,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ash: UI error: {e}");
            return 1;
        }
    };

    match result {
        // The audit passed — exec intentionally does not re-audit; the parse+audit above is
        // the single point of enforcement.
        ui::MenuResult::Execute => match exec::run(&draft.command) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("{e}");
                error::exit_code(&e)
            }
        },
        ui::MenuResult::Modify => {
            exec::echo_for_edit(&draft.command);
            0
        }
        ui::MenuResult::Cancel => 0,
    }
}
