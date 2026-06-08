use std::fmt;

#[derive(Debug)]
pub enum AshError {
    NoConfig,
    InvalidConfig(String),
    NetworkError(String),
    EnvProbeError(String),
    LlmOutputError(String),
}

impl fmt::Display for AshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoConfig => write!(f, "ERR-C001: no config file found"),
            Self::InvalidConfig(msg) => write!(f, "ERR-C002: {msg}"),
            Self::NetworkError(msg) => write!(f, "ERR-N001: {msg}"),
            Self::EnvProbeError(msg) => write!(f, "ERR-S001: {msg}"),
            Self::LlmOutputError(msg) => write!(f, "ERR-L001: {msg}"),
        }
    }
}

impl std::error::Error for AshError {}

pub fn exit_code(e: &AshError) -> i32 {
    match e {
        AshError::NoConfig | AshError::InvalidConfig(_) => 2,
        AshError::NetworkError(_) => 3,
        AshError::EnvProbeError(_) => 4,
        AshError::LlmOutputError(_) => 1,
    }
}
