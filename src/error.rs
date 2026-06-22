use std::fmt;

#[derive(Debug)]
pub enum AshError {
    NoConfig,
    InvalidConfig(String),
    NetworkError(String),
    EnvProbeError(String),
    LlmOutputError(String),
    Timeout(String),
    SymlinkConfig(String),
    ShellNotAllowlisted(String),
    ExecError(String),
}

impl fmt::Display for AshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoConfig => write!(f, "ERR-C001: no config file found"),
            Self::InvalidConfig(msg) => write!(f, "ERR-C002: {msg}"),
            Self::NetworkError(msg) => write!(f, "ERR-N001: {msg}"),
            Self::EnvProbeError(msg) => write!(f, "ERR-S001: {msg}"),
            Self::LlmOutputError(msg) => write!(f, "ERR-L001: {msg}"),
            Self::Timeout(msg) => write!(f, "ERR-N002: {msg}"),
            Self::SymlinkConfig(msg) => write!(f, "ERR-C003: {msg}"),
            Self::ShellNotAllowlisted(msg) => write!(f, "ERR-E001: {msg}"),
            Self::ExecError(msg) => write!(f, "ERR-E002: {msg}"),
        }
    }
}

impl std::error::Error for AshError {}

pub fn exit_code(e: &AshError) -> i32 {
    match e {
        AshError::NoConfig | AshError::InvalidConfig(_) | AshError::SymlinkConfig(_) => 2,
        AshError::NetworkError(_) | AshError::Timeout(_) => 3,
        AshError::EnvProbeError(_) => 4,
        AshError::LlmOutputError(_) => 1,
        AshError::ShellNotAllowlisted(_) | AshError::ExecError(_) => 5,
    }
}
