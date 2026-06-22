# 🌿 ash

> Turn natural language into audited shell commands — with a safety-first interactive TUI.

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org)

---

## ✨ What is ash?

**ash** is a CLI tool that takes a plain-English prompt, calls an LLM to generate the shell command, audits it against your allow/deny policy, then presents an interactive review menu — before anything runs.

```
$ ash "show last 10 git commits"

git log --oneline -10
↑342 ↓88 = 430 tok  audited  (show last 10 git commits)
✓[confirm]  ~[edit]  ✗[cancel]
```

You stay in control. Nothing executes without your say-so.

---

## 🔒 Security model

- **Allow / deny lists** — whitelist the commands you permit; blacklist what you never want
- **Argument-level audit** — known dangerous patterns (`rm -rf`, `git push --force`, `chmod 777`, `dd of=/dev/`, `curl | sh`, `mkfs`, …) are rejected regardless of the binary being allowlisted
- **Shell builtin awareness** — `cd`, `export`, `source`, `set`, … are recognized as valid even though they have no on-disk binary
- **Process substitution detection** — `$(...)`, backticks, and `<(…)` / `>(…)` are flagged as unparseable and require confirmation
- **Wrapper detection** — hard wrappers (`sudo`, `doas`, `su`, `nsenter`, `unshare`, `exec`, `sh`, `bash`, `zsh`, `fish`) always require double-confirmation; soft wrappers (`env`, `xargs`, `nohup`) require double-confirmation only when the segment contains shell metacharacters
- **Shell allowlist** — `$SHELL` is canonicalized and validated against a hardcoded list; unknown shells are rejected (no fallback)
- **Config symlink refusal** — config files that are symlinks are rejected to prevent TOCTOU and config injection
- **API key never logged** — `Config` deliberately omits `Debug`; key is redacted in all output
- **No `-i` flag** — interactive shell mode is intentionally omitted to prevent alias/function shadowing of audited commands
- **Audit transparency** — rejection reasons are always printed to stderr; never silently rejected

---

## 🚀 Installation

```bash
git clone https://github.com/yourname/ash
cd ash
cargo build --release
cp target/release/ash ~/.local/bin/
```

Requires Rust 1.88+ (edition 2024, let-chains).

---

## ⚙️ Configuration

Copy the example and fill in your values:

```bash
cp config.toml.example ~/.config/ash/config.toml
```

```toml
api_type      = "deepseek"
api_key       = "sk-..."       # or set ASH_API_KEY env var (or omit if env set)
model_name    = "deepseek-chat"

allow_list = []                # empty = all commands allowed
deny_list  = ["rm", "sudo", "dd", "mkfs"]

# request_timeout_secs = 60    # LLM request timeout
# tools_to_probe = ["git", "curl", "docker", "python3", "node", "cargo", "make"]
# collect_sys_info = true
# collect_env_info = true
```

**Supported providers:** `openai` · `anthropic` · `gemini` · `ollama` · `groq` · `cohere` · `deepseek` · `xai`

**API key via environment variable:**
```bash
export ASH_API_KEY="sk-..."
```

---

## 🎮 Usage

```bash
ash "list all running docker containers"
ash "find files larger than 100MB in /var"
ash "compress the logs directory"
```

### UI controls

| Key | Action |
|-----|--------|
| `←` / `→` or `Tab` | Navigate options |
| `Space` / `Enter` | Select |
| `Esc` / `Ctrl+C` | Cancel |

### Risk levels

| Label | Meaning |
|-------|---------|
| `audited` | Command passed all checks (binary allowlisted, no dangerous arguments) |
| 🟡 `mid risk` | Wrapper or unparseable syntax detected — double-confirm required |
| 🔴 `high risk` | Deny-list hit, dangerous argument pattern, or not found in PATH |

High-risk commands are always rejected. Rejection reasons are printed to stderr so you can see exactly why a command was blocked.

---

## 🛠️ Project structure

```
src/
├── main.rs            CLI entry point
├── config.rs          Config loading, validation, provider mapping
├── env_probe.rs       Safe environment summary for LLM context
├── llm.rs             genai client, prompt, JSON parsing
├── parser.rs          Shell-chain splitter, subshell detection
│   ├── scan.rs        Quote-aware splitting, subshell/process-sub detection
│   └── wrapper.rs     Hard/soft wrapper classification, metacharacter scan
├── risk.rs            Allow/deny/wrapper/path audit
│   ├── builtins.rs    Shell builtin allowlist (cd, export, source, …)
│   └── patterns.rs    Dangerous argument patterns, pipe-to-shell detection
├── ui.rs              Raw-mode TUI, double-confirm state machine
├── exec.rs            Shell execution, echo-for-edit
└── error.rs           Typed errors with stable codes (ERR-C001…)
```

---

## 🔧 Error codes

| Code | Meaning |
|------|---------|
| `ERR-C001` | No config file found |
| `ERR-C002` | Config invalid or missing required field |
| `ERR-C003` | Config file is a symlink (refused) |
| `ERR-N001` | Network / API / auth failure |
| `ERR-N002` | LLM request timed out |
| `ERR-S001` | Environment collection failed |
| `ERR-L001` | LLM returned invalid or empty output |
| `ERR-E001` | Shell not allowlisted or canonicalization failed |
| `ERR-E002` | Command execution failed |

---

## 📄 License

ash is free software: you can redistribute it and/or modify it under the terms of the [GNU General Public License v3.0](LICENSE).
