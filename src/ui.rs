use crate::risk::RiskLevel;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuItem {
    Confirm,
    Modify,
    Cancel,
}

const ITEMS: [MenuItem; 3] = [MenuItem::Confirm, MenuItem::Modify, MenuItem::Cancel];

#[derive(Debug, PartialEq)]
pub enum MenuResult {
    Execute,
    Modify,
    Cancel,
}

pub struct UiContext {
    pub tokens_in: Option<i64>,
    pub tokens_out: Option<i64>,
    pub tokens_total: Option<i64>,
    pub risk_level: RiskLevel,
}

pub fn run_menu(
    command: &str,
    explanation: Option<&str>,
    ctx: &UiContext,
    need_double_confirm: bool,
    high_risk: bool,
) -> io::Result<MenuResult> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    // RawGuard ensures raw mode is always restored, even on panic or early error return.
    struct RawGuard;
    impl Drop for RawGuard {
        fn drop(&mut self) {
            let _ = terminal::disable_raw_mode();
        }
    }
    let _guard = RawGuard;

    let mut selected = 0usize;
    let mut confirm_armed = false;

    execute!(stdout, cursor::MoveToColumn(0))?;
    print_header(&mut stdout, command, explanation, ctx, high_risk)?;

    let result = loop {
        draw_menu(
            &mut stdout,
            selected,
            confirm_armed,
            need_double_confirm || high_risk,
        )?;

        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
            match code {
                KeyCode::Left | KeyCode::Up => {
                    selected = (selected + ITEMS.len() - 1) % ITEMS.len();
                    if ITEMS[selected] != MenuItem::Confirm {
                        confirm_armed = false;
                    }
                }
                KeyCode::Right | KeyCode::Down | KeyCode::Tab => {
                    selected = (selected + 1) % ITEMS.len();
                    if ITEMS[selected] != MenuItem::Confirm {
                        confirm_armed = false;
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => match ITEMS[selected] {
                    MenuItem::Confirm => {
                        if need_double_confirm || high_risk {
                            if confirm_armed {
                                break MenuResult::Execute;
                            } else {
                                confirm_armed = true;
                            }
                        } else {
                            break MenuResult::Execute;
                        }
                    }
                    MenuItem::Modify => break MenuResult::Modify,
                    MenuItem::Cancel => break MenuResult::Cancel,
                },
                KeyCode::Esc => break MenuResult::Cancel,
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    break MenuResult::Cancel;
                }
                _ => {}
            }
        }
    };

    // Clear the menu line and move to a clean new line.
    execute!(
        stdout,
        cursor::MoveToColumn(0),
        terminal::Clear(ClearType::CurrentLine),
        Print("\n")
    )?;
    Ok(result)
}

fn print_header(
    stdout: &mut io::Stdout,
    command: &str,
    explanation: Option<&str>,
    ctx: &UiContext,
    high_risk: bool,
) -> io::Result<()> {
    if high_risk {
        execute!(
            stdout,
            SetForegroundColor(Color::Red),
            Print("⚠ HIGH RISK  "),
            ResetColor
        )?;
    }
    execute!(
        stdout,
        SetForegroundColor(Color::Cyan),
        Print(command),
        ResetColor,
        Print("\n")
    )?;

    let fmt = |n: Option<i64>| n.map_or_else(|| "-".to_string(), |v| v.to_string());
    let (risk_color, risk_label) = match ctx.risk_level {
        RiskLevel::Safe => (Color::DarkGrey, "audited"),
        RiskLevel::Mid => (Color::Yellow, "mid risk"),
        RiskLevel::High => (Color::Red, "high risk"),
    };

    execute!(
        stdout,
        cursor::MoveToColumn(0),
        SetForegroundColor(Color::DarkGrey),
        Print(format!(
            "↑{} ↓{} = {} tok  ",
            fmt(ctx.tokens_in),
            fmt(ctx.tokens_out),
            fmt(ctx.tokens_total),
        )),
        SetForegroundColor(risk_color),
        Print(risk_label),
        ResetColor,
        SetForegroundColor(Color::DarkGrey)
    )?;
    if let Some(exp) = explanation {
        execute!(stdout, Print(format!("  ({exp})")))?;
    }
    execute!(stdout, Print("\n"), ResetColor)?;
    Ok(())
}

fn draw_menu(
    stdout: &mut io::Stdout,
    selected: usize,
    confirm_armed: bool,
    double_confirm: bool,
) -> io::Result<()> {
    execute!(
        stdout,
        cursor::MoveToColumn(0),
        terminal::Clear(ClearType::CurrentLine)
    )?;

    for (i, item) in ITEMS.iter().enumerate() {
        let (icon, icon_color, label) = match item {
            MenuItem::Confirm if double_confirm && confirm_armed => {
                ("✓", Color::Green, "[confirm ✓]")
            }
            MenuItem::Confirm => ("✓", Color::Green, "[confirm]"),
            MenuItem::Modify => ("~", Color::Yellow, "[edit]"),
            MenuItem::Cancel => ("✗", Color::Red, "[cancel]"),
        };
        let text_color = if i == selected {
            Color::Green
        } else {
            Color::Reset
        };
        execute!(
            stdout,
            SetForegroundColor(icon_color),
            Print(icon),
            SetForegroundColor(text_color),
            Print(format!("{label} ")),
            ResetColor
        )?;
    }
    stdout.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    fn ring_next(sel: usize, len: usize) -> usize {
        (sel + 1) % len
    }
    fn ring_prev(sel: usize, len: usize) -> usize {
        (sel + len - 1) % len
    }

    #[test]
    fn ring_navigation() {
        assert_eq!(ring_next(2, 3), 0);
        assert_eq!(ring_prev(0, 3), 2);
        assert_eq!(ring_next(0, 3), 1);
        assert_eq!(ring_prev(1, 3), 0);
    }
}
