#[cfg(not(target_arch = "wasm32"))]
pub type Instant = std::time::Instant;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant;

#[cfg(target_arch = "wasm32")]
impl Instant {
    pub fn now() -> Self {
        Self
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.duration_since(*self)
    }

    pub fn duration_since(&self, _earlier: Self) -> std::time::Duration {
        std::time::Duration::ZERO
    }
}

#[cfg(target_arch = "wasm32")]
impl std::ops::Add<std::time::Duration> for Instant {
    type Output = Instant;

    fn add(self, _rhs: std::time::Duration) -> Instant {
        self
    }
}
