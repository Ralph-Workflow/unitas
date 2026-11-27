use std::cell::RefCell;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

const CAPTURE_MAGIC: u32 = 0x5052_4643;
const CAPTURE_VERSION: u32 = 1;
const ROOT_NAME: &str = "Capture";
const FRAME_NAME: &str = "Frame";

static RECORDING: AtomicBool = AtomicBool::new(false);
static CAPTURE: Mutex<Capture> = Mutex::new(Capture::new());

thread_local! {
    static STACK: RefCell<Vec<OpenMarker>> = const { RefCell::new(Vec::new()) };
}

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

struct MarkerNode {
    name: String,
    duration_ns: u64,
    children: Vec<usize>,
}

impl MarkerNode {
    const fn new() -> Self {
        Self {
            name: String::new(),
            duration_ns: 0,
            children: Vec::new(),
        }
    }
}

struct OpenMarker {
    index: usize,
    start: Instant,
}

struct Capture {
    recording: bool,
    root: MarkerNode,
    nodes: Vec<MarkerNode>,
    scratch: Vec<u8>,
    bytes: Vec<u8>,
}

impl Capture {
    const fn new() -> Self {
        Self {
            recording: false,
            root: MarkerNode::new(),
            nodes: Vec::new(),
            scratch: Vec::new(),
            bytes: Vec::new(),
        }
    }

    fn begin(&mut self, name: &str) -> Option<usize> {
        if !self.recording {
            return None;
        }
        let index = self.nodes.len();
        self.nodes.push(MarkerNode {
            name: name.to_string(),
            duration_ns: 0,
            children: Vec::new(),
        });
        let parent = STACK.with(|stack| stack.borrow().last().map(|open| open.index));
        match parent {
            Some(parent) => {
                if let Some(node) = self.nodes.get_mut(parent) {
                    node.children.push(index);
                }
            }
            None => self.root.children.push(index),
        }
        let start = Instant::now();
        STACK.with(|stack| stack.borrow_mut().push(OpenMarker { index, start }));
        Some(index)
    }

    fn end(&mut self, index: usize) {
        let elapsed = STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            let mut elapsed = None;
            while let Some(open) = stack.pop() {
                if open.index == index {
                    elapsed = Some(open.start.elapsed().as_nanos());
                    break;
                }
            }
            elapsed
        });
        if let Some(elapsed) = elapsed
            && let Some(node) = self.nodes.get_mut(index)
        {
            node.duration_ns = u64::try_from(elapsed).unwrap_or(u64::MAX);
        }
    }

    fn end_top(&mut self) {
        let closed = STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            stack
                .pop()
                .map(|open| (open.index, open.start.elapsed().as_nanos()))
        });
        if let Some((index, elapsed)) = closed
            && let Some(node) = self.nodes.get_mut(index)
        {
            node.duration_ns = u64::try_from(elapsed).unwrap_or(u64::MAX);
        }
    }

    fn count(&self, index: usize) -> u32 {
        let mut total = 1u32;
        if let Some(marker) = self.nodes.get(index) {
            for child in &marker.children {
                total += self.count(*child);
            }
        }
        total
    }

    fn write(&self, index: usize, depth: u32, bytes: &mut Vec<u8>) {
        let Some(marker) = self.nodes.get(index) else {
            return;
        };
        bytes.extend_from_slice(&depth.to_le_bytes());
        bytes.extend_from_slice(&marker.duration_ns.to_le_bytes());
        let name = marker.name.as_bytes();
        bytes.extend_from_slice(&u32::try_from(name.len()).unwrap_or(0).to_le_bytes());
        bytes.extend_from_slice(name);
        for child in &marker.children {
            self.write(*child, depth + 1, bytes);
        }
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&CAPTURE_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&CAPTURE_VERSION.to_le_bytes());
        bytes.extend_from_slice(&self.count(0).to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&self.root.duration_ns.to_le_bytes());
        let name = self.root.name.as_bytes();
        bytes.extend_from_slice(&u32::try_from(name.len()).unwrap_or(0).to_le_bytes());
        bytes.extend_from_slice(name);
        for child in &self.root.children {
            self.write(*child, 1, &mut bytes);
        }
        bytes
    }
}

fn with_capture<T>(action: impl FnOnce(&mut Capture) -> T) -> Option<T> {
    let mut guard = CAPTURE.lock().ok()?;
    Some(action(&mut guard))
}

/// Whether a capture is running, so the player loop can skip its timing work.
pub fn recording() -> bool {
    RECORDING.load(Ordering::Relaxed)
}

/// Opens the marker that holds one frame of the player loop.
pub fn begin_frame() -> Option<usize> {
    if !recording() {
        return None;
    }
    with_capture(|capture| capture.begin(FRAME_NAME)).flatten()
}

/// Closes the marker [`begin_frame`] opened.
pub fn end_frame(marker: Option<usize>) {
    let Some(index) = marker else {
        return;
    };
    with_capture(|capture| capture.end(index));
}

/// Opens the marker that holds one player loop phase.
pub fn begin_phase(name: &str) -> Option<usize> {
    if !recording() {
        return None;
    }
    with_capture(|capture| capture.begin(name)).flatten()
}

/// Closes the marker [`begin_phase`] opened.
pub fn end_phase(marker: Option<usize>) {
    let Some(index) = marker else {
        return;
    };
    with_capture(|capture| capture.end(index));
}

/// Starts a capture, dropping the markers of an earlier one.
pub fn start_capture() {
    with_capture(|capture| {
        capture.recording = true;
        capture.root = MarkerNode::new();
        capture.root.name = ROOT_NAME.to_string();
        capture.nodes.clear();
        capture.scratch.clear();
        capture.bytes.clear();
    });
    RECORDING.store(true, Ordering::Relaxed);
}

/// Stops the capture, keeping its markers readable.
pub fn stop_capture() {
    RECORDING.store(false, Ordering::Relaxed);
    with_capture(|capture| {
        capture.recording = false;
    });
}

/// The captured marker tree in the byte form the managed bridge reads.
pub fn capture_bytes() -> Vec<u8> {
    with_capture(|capture| capture.serialize()).unwrap_or_default()
}

fn read_scratch(address: u64, length: u64) -> Option<String> {
    let capture = CAPTURE.lock().ok()?;
    let start = usize::try_from(address).ok()?;
    let len = usize::try_from(length).ok()?;
    let base = capture.scratch.as_ptr() as usize;
    let offset = start.checked_sub(base)?;
    let end = offset.checked_add(len)?;
    if end > capture.scratch.len() {
        return None;
    }
    String::from_utf8(capture.scratch[offset..end].to_vec()).ok()
}

ffi_export! {
pub extern "C" fn unitas_runtime_profiler_start_capture() -> i32 {
    start_capture();
    0
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_profiler_stop_capture() -> i32 {
    stop_capture();
    0
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_profiler_capture_size() -> u64 {
    let bytes = capture_bytes();
    let mut capture = match CAPTURE.lock() {
        Ok(capture) => capture,
        Err(_) => return 0,
    };
    capture.bytes.clear();
    capture.bytes.extend_from_slice(&bytes);
    u64::try_from(capture.bytes.len()).unwrap_or(0)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_profiler_capture_buffer(capacity: u64) -> u64 {
    let cap = match usize::try_from(capacity) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    if cap == 0 {
        return 0;
    }
    let mut capture = match CAPTURE.lock() {
        Ok(capture) => capture,
        Err(_) => return 0,
    };
    if capture.bytes.len() < cap {
        capture.bytes.resize(cap, 0);
    }
    capture.bytes.as_ptr() as usize as u64
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_profiler_scratch_buffer(capacity: u64) -> u64 {
    let cap = match usize::try_from(capacity) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    if cap == 0 {
        return 0;
    }
    let mut capture = match CAPTURE.lock() {
        Ok(capture) => capture,
        Err(_) => return 0,
    };
    if capture.scratch.len() < cap {
        capture.scratch.resize(cap, 0);
    }
    capture.scratch.as_ptr() as usize as u64
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_profiler_begin_sample(address: u64, length: u64) -> i32 {
    let Some(name) = read_scratch(address, length) else {
        return -2;
    };
    with_capture(|capture| {
        capture.begin(&name);
    });
    0
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_profiler_end_sample() -> i32 {
    with_capture(|capture| {
        capture.end_top();
    });
    0
}
}

#[cfg(test)]
mod tests {
    use crate::player_loop::{
        EARLY_UPDATE, FIXED_UPDATE, INITIALIZATION, POST_LATE_UPDATE, PRE_LATE_UPDATE, PRE_UPDATE,
        UPDATE,
    };
    use crate::runtime::Runtime;

    use super::{capture_bytes, start_capture, stop_capture};

    #[test]
    fn a_tick_records_the_player_loop_phases_in_order() -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime = Runtime::new()?;
        runtime.tick(0.02)?;
        start_capture();
        runtime.tick(0.02)?;
        stop_capture();
        let bytes = capture_bytes();
        let frames = parse_frames(&bytes);
        let expected = [
            INITIALIZATION,
            EARLY_UPDATE,
            FIXED_UPDATE,
            PRE_UPDATE,
            UPDATE,
            PRE_LATE_UPDATE,
            POST_LATE_UPDATE,
        ];
        let recorded = frames.iter().any(|frame| {
            frame.len() == expected.len()
                && frame
                    .iter()
                    .zip(expected)
                    .all(|(name, phase)| name == phase)
        });
        assert!(recorded, "the tick recorded the Unity phases in order");
        Ok(())
    }

    fn parse_frames(bytes: &[u8]) -> Vec<Vec<String>> {
        let mut cursor = 12usize;
        let mut frames: Vec<Vec<String>> = Vec::new();
        while cursor < bytes.len() {
            let Some(depth) = read_u32(bytes, &mut cursor) else {
                break;
            };
            let Some(_duration) = read_u64(bytes, &mut cursor) else {
                break;
            };
            let Some(name) = read_string(bytes, &mut cursor) else {
                break;
            };
            match depth {
                1 => frames.push(Vec::new()),
                2 => {
                    if let Some(frame) = frames.last_mut() {
                        frame.push(name);
                    }
                }
                _ => {}
            }
        }
        frames
    }

    fn read_u32(bytes: &[u8], cursor: &mut usize) -> Option<u32> {
        let end = cursor.checked_add(4)?;
        let raw = bytes.get(*cursor..end)?;
        *cursor = end;
        Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
    }

    fn read_u64(bytes: &[u8], cursor: &mut usize) -> Option<u64> {
        let end = cursor.checked_add(8)?;
        let raw = bytes.get(*cursor..end)?;
        *cursor = end;
        let mut value = [0u8; 8];
        value.copy_from_slice(raw);
        Some(u64::from_le_bytes(value))
    }

    fn read_string(bytes: &[u8], cursor: &mut usize) -> Option<String> {
        let length = read_u32(bytes, cursor)? as usize;
        let end = cursor.checked_add(length)?;
        let raw = bytes.get(*cursor..end)?;
        *cursor = end;
        String::from_utf8(raw.to_vec()).ok()
    }
}
