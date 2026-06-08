use ash::{config, env_probe, error, exec, llm, parser, risk, ui};
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: ash <prompt...>");
        std::process::exit(1);
    }
    let prompt = args.join(" ");
    std::process::exit(run(&prompt).await);
}

async fn run(prompt: &str) -> i32 {
    macro_rules! or_bail {
        ($expr:expr) => {
            match $expr {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e}");
                    return error::exit_code(&e);
                }
            }
        };
    }

    let cfg = or_bail!(config::load());
    let env_summary = or_bail!(env_probe::collect(
        cfg.collect_sys_info,
        cfg.collect_env_info
    ));
    let (draft, usage) = or_bail!(llm::generate(prompt, &env_summary, &cfg).await);

    let parse_result = parser::parse(&draft.command);
    let audit = risk::audit(&parse_result, &cfg);

    // Hard block: rejected commands never execute regardless of silent_reject.
    if audit.should_reject {
        if cfg.silent_reject {
            eprintln!("ash: command rejected (security policy)");
        } else {
            // Show UI so the user sees what was blocked, but Execute is disabled.
            let ctx = ui::UiContext {
                tokens_in: usage.prompt_tokens,
                tokens_out: usage.completion_tokens,
                tokens_total: usage.total_tokens,
                risk_level: audit.risk_level,
            };
            let _ = ui::run_menu(
                &draft.command,
                draft.explanation.as_deref(),
                &ctx,
                true,
                true,
            );
            eprintln!("ash: command rejected (security policy)");
        }
        return 1;
    }

    let need_double = audit.need_double_confirm;

    let ctx = ui::UiContext {
        tokens_in: usage.prompt_tokens,
        tokens_out: usage.completion_tokens,
        tokens_total: usage.total_tokens,
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
        ui::MenuResult::Execute => exec::run(&draft.command),
        ui::MenuResult::Modify => {
            exec::echo_for_edit(&draft.command);
            0
        }
        ui::MenuResult::Cancel => 0,
    }
}
