use std::cell::RefCell;

use crate::error::RuntimeError;
use crate::runtime::Runtime;

pub const WEB_BOOT_OK: i32 = 0;
pub const WEB_BOOT_FAILED: i32 = -1;
pub const WEB_STATE_UNAVAILABLE: i32 = -2;
pub const WEB_NOT_BOOTED: i32 = -3;
pub const WEB_TICK_FAILED: i32 = -4;

pub struct WebRuntime {
    runtime: Runtime,
    frame_count: u64,
}

impl WebRuntime {
    pub fn new() -> Result<Self, RuntimeError> {
        Ok(Self {
            runtime: Runtime::new()?,
            frame_count: 0,
        })
    }

    pub fn tick(&mut self, delta_seconds: f64) -> Result<(), RuntimeError> {
        self.runtime.tick(delta_seconds)?;
        self.frame_count += 1;
        Ok(())
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

thread_local! {
    static SLOT: RefCell<Option<WebRuntime>> = const { RefCell::new(None) };
}

pub fn boot() -> i32 {
    SLOT.with(|cell| {
        let Ok(mut slot) = cell.try_borrow_mut() else {
            return WEB_STATE_UNAVAILABLE;
        };
        if slot.is_some() {
            return WEB_BOOT_OK;
        }
        match WebRuntime::new() {
            Ok(runtime) => {
                *slot = Some(runtime);
                WEB_BOOT_OK
            }
            Err(_) => WEB_BOOT_FAILED,
        }
    })
}

pub fn tick(delta_seconds: f64) -> i32 {
    SLOT.with(|cell| {
        let Ok(mut slot) = cell.try_borrow_mut() else {
            return WEB_STATE_UNAVAILABLE;
        };
        let Some(runtime) = slot.as_mut() else {
            return WEB_NOT_BOOTED;
        };
        match runtime.tick(delta_seconds) {
            Ok(()) => WEB_BOOT_OK,
            Err(_) => WEB_TICK_FAILED,
        }
    })
}

pub fn frame_count() -> f64 {
    SLOT.with(|cell| {
        let Ok(slot) = cell.try_borrow() else {
            return 0.0;
        };
        slot.as_ref()
            .map_or(0.0, |runtime| runtime.frame_count() as f64)
    })
}

#[cfg(target_arch = "wasm32")]
ffi_export! {
pub extern "C" fn unitas_web_boot() -> i32 {
    boot()
}
}

#[cfg(target_arch = "wasm32")]
ffi_export! {
pub extern "C" fn unitas_web_tick(delta_seconds: f64) -> i32 {
    tick(delta_seconds)
}
}

#[cfg(target_arch = "wasm32")]
ffi_export! {
pub extern "C" fn unitas_web_frame_count() -> f64 {
    frame_count()
}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() {
        SLOT.with(|cell| {
            if let Ok(mut slot) = cell.try_borrow_mut() {
                *slot = None;
            }
        });
    }

    #[test]
    fn boot_is_idempotent_and_keeps_the_running_session() {
        reset();
        assert_eq!(boot(), WEB_BOOT_OK);
        assert_eq!(tick(0.016), WEB_BOOT_OK);
        assert_eq!(frame_count(), 1.0);
        assert_eq!(boot(), WEB_BOOT_OK);
        assert_eq!(frame_count(), 1.0);
    }

    #[test]
    fn tick_before_boot_fails() {
        reset();
        assert_eq!(tick(0.016), WEB_NOT_BOOTED);
        assert_eq!(frame_count(), 0.0);
    }

    #[test]
    fn every_completed_tick_increments_the_frame_count() {
        reset();
        assert_eq!(boot(), WEB_BOOT_OK);
        assert_eq!(frame_count(), 0.0);
        assert_eq!(tick(0.016), WEB_BOOT_OK);
        assert_eq!(tick(0.016), WEB_BOOT_OK);
        assert_eq!(frame_count(), 2.0);
    }

    #[test]
    fn a_fresh_runtime_starts_without_frames() {
        let runtime = match WebRuntime::new() {
            Ok(runtime) => runtime,
            Err(_) => return,
        };
        assert_eq!(runtime.frame_count(), 0);
    }
}
