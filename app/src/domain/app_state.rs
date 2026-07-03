#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tone {
    Info,
    Ok,
    Warn,
    Error,
}

impl Tone {
    pub(crate) fn class(self) -> &'static str {
        match self {
            Tone::Info => "info",
            Tone::Ok => "ok",
            Tone::Warn => "warn",
            Tone::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Status {
    pub(crate) message: String,
    pub(crate) tone: Tone,
}

impl Status {
    pub(crate) fn new(message: impl Into<String>, tone: Tone) -> Self {
        Self {
            message: message.into(),
            tone,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OpenTabError {
    Read(String),
    Decode(String),
}

impl OpenTabError {
    pub(crate) fn message(self) -> String {
        match self {
            Self::Read(message) | Self::Decode(message) => message,
        }
    }
}
