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
↑342 ↓88 = 430 tok  safe risk  (show last 10 git commits)
✓[confirm]  ~[edit]  ✗[cancel]
```

You stay in control. Nothing executes without your say-so.

---

## 🔒 Security model

- **Allow / deny lists** — whitelist the commands you permit; blacklist what you never want
- **Wrapper detection** — `sudo`, `doas`, `xargs`, `nsenter` and others always require double-confirmation
- **Subshell detection** — `$(...)` and backtick syntax are flagged as unparseable and require confirmation
- **Shell allowlist** — `$SHELL` is validated against a hardcoded list; unknown shells fall back to `/bin/sh`
- **API key never logged** — `Config` deliberately omits `Debug`; key is redacted in all output
- **No `-i` flag** — interactive shell mode is intentionally omitted to prevent alias/function shadowing of audited commands

---

## 🚀 Installation

```bash
git clone https://github.com/yourname/ash
cd ash
cargo build --release
cp target/release/ash ~/.local/bin/
```

Requires Rust 1.85+ (edition 2024).

---

## ⚙️ Configuration

Copy the example and fill in your values:

```bash
cp config.toml.example ~/.config/ash/config.toml
```

```toml
provider_name = "deepseek"
api_type      = "deepseek"
api_key       = "sk-..."       # or set ASH_API_KEY env var
model_name    = "deepseek-chat"

allow_list = []                # empty = all commands allowed
deny_list  = ["rm", "sudo", "dd", "mkfs"]

# silent_reject = true         # false = show UI for rejected commands
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
| 🟢 `safe risk` | Command passed all checks |
| 🟡 `mid risk` | Wrapper detected — double-confirm required |
| 🔴 `high risk` | Deny-list hit or not found in PATH |

When `silent_reject = true` (default), high-risk commands exit immediately. Set to `false` to show the UI and let you review before blocking.

---

## 🛠️ Project structure

```
src/
├── main.rs       CLI entry point
├── config.rs     Config loading, validation, provider mapping
├── env_probe.rs  Safe environment summary for LLM context
├── llm.rs        genai client, prompt, JSON parsing
├── parser.rs     Shell-chain splitter, subshell detection
├── risk.rs       Allow/deny/wrapper/path audit
├── ui.rs         Raw-mode TUI, double-confirm state machine
├── exec.rs       Shell execution, echo-for-edit
└── error.rs      Typed errors with stable codes (ERR-C001…)
```

---

## 🔧 Error codes

| Code | Meaning |
|------|---------|
| `ERR-C001` | No config file found |
| `ERR-C002` | Config invalid or missing required field |
| `ERR-N001` | Network / API / auth failure |
| `ERR-S001` | Environment collection failed |
| `ERR-L001` | LLM returned invalid or empty output |

---

## 📄 License

ash is free software: you can redistribute it and/or modify it under the terms of the [GNU General Public License v3.0](LICENSE).
