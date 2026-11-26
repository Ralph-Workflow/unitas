use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

macro_rules! ffi_export {
    (pub extern "C" fn $name:ident($($arg:ident : $arg_ty:ty),* $(,)?) $(-> $ret:ty)? $body:block) => {
        #[unsafe(export_name = stringify!($name))]
        pub extern "C" fn $name($($arg : $arg_ty),*) $(-> $ret)? {
            #[used]
            static KEEP: extern "C" fn($($arg_ty),*) $(-> $ret)? = $name;
            $body
        }
    };
}

static PLAYER_HUD_BUFFERS: OnceLock<Mutex<HashMap<u64, Vec<u8>>>> = OnceLock::new();

fn player_hud_buffers() -> &'static Mutex<HashMap<u64, Vec<u8>>> {
    PLAYER_HUD_BUFFERS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn write_player_hud_buffer(handle: u64, text: &str) -> Option<u64> {
    let Ok(mut buffers) = player_hud_buffers().lock() else {
        return None;
    };
    let buffer = buffers.entry(handle).or_default();
    buffer.clear();
    buffer.extend_from_slice(text.as_bytes());
    buffer.push(0);
    Some(buffer.as_ptr() as usize as u64)
}

ffi_export! {
pub extern "C" fn unitas_runtime_player_publish_hud(handle: u64, address: u64, length: u64) -> i32 {
    let Ok(length) = usize::try_from(length) else {
        return -2;
    };
    let Ok(mut buffers) = player_hud_buffers().lock() else {
        return -1;
    };
    let Some(buffer) = buffers.get_mut(&handle) else {
        return -1;
    };
    if address != buffer.as_ptr() as usize as u64 {
        return -2;
    }
    if length > buffer.len() {
        return -2;
    }
    buffer.truncate(length);
    0
}
}

pub fn player_hud_text(handle: u64) -> Option<String> {
    let Ok(buffers) = player_hud_buffers().lock() else {
        return None;
    };
    let buffer = buffers.get(&handle)?;
    let end = buffer
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(buffer.len());
    Some(String::from_utf8_lossy(&buffer[..end]).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_hud_text_round_trips_through_the_player_abi() {
        let handle = 41_001;
        let address = write_player_hud_buffer(handle, "Score: 3/12").unwrap_or(0);
        assert_ne!(address, 0);
        assert_eq!(unitas_runtime_player_publish_hud(handle, address, 11), 0);
        assert_eq!(player_hud_text(handle).as_deref(), Some("Score: 3/12"));
    }

    #[test]
    fn publishing_with_a_foreign_address_is_refused() {
        let handle = 41_002;
        let address = write_player_hud_buffer(handle, "Score: 1/12").unwrap_or(0);
        assert_eq!(
            unitas_runtime_player_publish_hud(handle, address + 8, 11),
            -2
        );
        assert_eq!(unitas_runtime_player_publish_hud(41_003, address, 11), -1);
    }
}
