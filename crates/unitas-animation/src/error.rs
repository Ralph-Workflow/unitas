use crate::controller::AnimationDiagnostic;

#[derive(Clone, Debug, PartialEq)]
pub enum AnimationError {
    Diagnostic(AnimationDiagnostic),
    UnexpectedEof,
    InvalidMagic,
    UnsupportedVersion(u32),
    InvalidUtf8,
    InvalidTag { section: &'static str, tag: u8 },
}

impl std::fmt::Display for AnimationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Diagnostic(diagnostic) => write!(formatter, "{diagnostic}"),
            Self::UnexpectedEof => write!(formatter, "unexpected end of controller wire buffer"),
            Self::InvalidMagic => write!(formatter, "invalid controller wire magic header"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported controller wire version {version}")
            }
            Self::InvalidUtf8 => write!(formatter, "invalid utf-8 string in controller wire buffer"),
            Self::InvalidTag { section, tag } => {
                write!(formatter, "invalid tag {tag} in controller wire section {section}")
            }
        }
    }
}

impl std::error::Error for AnimationError {}

impl From<AnimationDiagnostic> for AnimationError {
    fn from(diagnostic: AnimationDiagnostic) -> Self {
        Self::Diagnostic(diagnostic)
    }
}
