use crate::controller::AnimationDiagnostic;

#[derive(Clone, Debug, PartialEq)]
pub enum AnimationError {
    Diagnostic(AnimationDiagnostic),
    InvalidDeltaTime(f32),
    UnexpectedEof,
    InvalidMagic,
    UnsupportedVersion(u32),
    InvalidUtf8,
    InvalidTag { section: &'static str, tag: u8 },
    InvalidStateIndex(usize),
}

impl std::fmt::Display for AnimationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Diagnostic(diagnostic) => write!(formatter, "{diagnostic}"),
            Self::InvalidDeltaTime(dt) => {
                write!(formatter, "delta seconds {dt} is not a finite non-negative number")
            }
            Self::UnexpectedEof => write!(formatter, "unexpected end of controller wire buffer"),
            Self::InvalidMagic => write!(formatter, "invalid controller wire magic header"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported controller wire version {version}")
            }
            Self::InvalidUtf8 => write!(formatter, "invalid utf-8 string in controller wire buffer"),
            Self::InvalidTag { section, tag } => {
                write!(formatter, "invalid tag {tag} in controller wire section {section}")
            }
            Self::InvalidStateIndex(index) => {
                write!(formatter, "invalid state index {index} in controller wire buffer")
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
