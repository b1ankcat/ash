mod scan;
mod wrapper;

pub use wrapper::{is_hard_wrapper, is_soft_wrapper};
pub(crate) use scan::{contains_subshell, extract_cmd_and_args, extract_primary, split_on_operators};
pub(crate) use wrapper::has_unquoted_metacharacter;

/// Result of parsing a shell command string.
pub struct ParseResult {
    pub commands: Vec<String>,
    pub segments: Vec<String>,
    pub has_unparseable: bool,
}

pub fn parse(input: &str) -> ParseResult {
    let raw_segments = split_on_operators(input);
    let mut commands = Vec::new();
    let mut segments = Vec::new();
    let mut has_unparseable = false;

    for seg in raw_segments {
        let trimmed = seg.trim();
        if trimmed.is_empty() {
            continue;
        }
        match extract_primary(trimmed) {
            Some(cmd) => {
                commands.push(cmd);
                segments.push(trimmed.to_string());
            }
            None => has_unparseable = true,
        }
    }

    // Subshell syntax is never safe to execute without re-auditing the inner command.
    if contains_subshell(input) {
        has_unparseable = true;
    }

    ParseResult {
        commands,
        segments,
        has_unparseable,
    }
}
