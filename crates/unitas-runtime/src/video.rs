use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::ffi::{CString, c_char, c_double, c_int, c_uint, c_void};
use std::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use libloading::Library;
use unitas_asset::video_import::{CookedVideo, load_or_cook_video};
use unitas_core::EntityId;

use crate::runtime::Runtime;
use crate::time::Instant;

#[cfg(not(target_arch = "wasm32"))]
const AVMEDIA_TYPE_VIDEO: c_int = 0;
#[cfg(not(target_arch = "wasm32"))]
const AV_PIX_FMT_RGBA: c_int = 26;
#[cfg(not(target_arch = "wasm32"))]
const SWS_BILINEAR: c_int = 2;

static VIDEO_PLAYER_ID_SEQ: AtomicU64 = AtomicU64::new(1);
#[cfg(not(target_arch = "wasm32"))]
static VIDEO_TEMP_SEQ: AtomicU64 = AtomicU64::new(1);
static VIDEO_PLAYERS: OnceLock<Mutex<BTreeMap<u64, Arc<Mutex<VideoPlayerInternal>>>>> =
    OnceLock::new();

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VideoDecodeError {
    PlatformLimited(String),
    LibraryUnavailable(String),
    IoError(String),
    CorruptStream(String),
    UnsupportedCodec(String),
}

impl fmt::Display for VideoDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlatformLimited(detail) => write!(formatter, "platform-limited video: {detail}"),
            Self::LibraryUnavailable(detail) => {
                write!(formatter, "FFmpeg libraries unavailable: {detail}")
            }
            Self::IoError(detail) => write!(formatter, "video I/O failed: {detail}"),
            Self::CorruptStream(detail) => write!(formatter, "corrupt video stream: {detail}"),
            Self::UnsupportedCodec(detail) => {
                write!(formatter, "unsupported video codec: {detail}")
            }
        }
    }
}

impl std::error::Error for VideoDecodeError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedVideoClip {
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub duration_micros: u64,
    pub frames_rgba: Vec<Vec<u8>>,
}

impl DecodedVideoClip {
    pub fn frame_count(&self) -> u64 {
        self.frames_rgba.len() as u64
    }

    pub fn frame_rate(&self) -> f64 {
        if self.fps_den == 0 {
            0.0
        } else {
            f64::from(self.fps_num) / f64::from(self.fps_den)
        }
    }

    pub fn duration_seconds(&self) -> f64 {
        if self.duration_micros > 0 {
            self.duration_micros as f64 / 1_000_000.0
        } else {
            let rate = self.frame_rate();
            if rate > 0.0 {
                self.frames_rgba.len() as f64 / rate
            } else {
                0.0
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VideoEvents {
    pub prepare_completed: bool,
    pub started: bool,
    pub frame_ready: bool,
    pub loop_point_reached: bool,
    pub seek_completed: bool,
    pub error_received: bool,
}

impl VideoEvents {
    pub fn to_bits(self) -> u32 {
        let mut bits = 0u32;
        if self.prepare_completed {
            bits |= 1 << 0;
        }
        if self.started {
            bits |= 1 << 1;
        }
        if self.frame_ready {
            bits |= 1 << 2;
        }
        if self.loop_point_reached {
            bits |= 1 << 3;
        }
        if self.seek_completed {
            bits |= 1 << 4;
        }
        if self.error_received {
            bits |= 1 << 5;
        }
        bits
    }

    pub fn from_bits(bits: u32) -> Self {
        Self {
            prepare_completed: (bits & (1 << 0)) != 0,
            started: (bits & (1 << 1)) != 0,
            frame_ready: (bits & (1 << 2)) != 0,
            loop_point_reached: (bits & (1 << 3)) != 0,
            seek_completed: (bits & (1 << 4)) != 0,
            error_received: (bits & (1 << 5)) != 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VideoPlayerSnapshot {
    pub width: u32,
    pub height: u32,
    pub frame_count: u64,
    pub frame_rate: f64,
    pub length_seconds: f64,
    pub current_frame: i64,
    pub current_time: f64,
    pub playback_speed: f32,
    pub is_prepared: bool,
    pub is_playing: bool,
    pub is_paused: bool,
    pub is_looping: bool,
    pub send_frame_ready_events: bool,
    pub events: VideoEvents,
    pub last_error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoPlayerState {
    Idle,
    Preparing,
    Prepared,
    Playing,
    Paused,
    Stopped,
    Failed,
}

enum WorkerStatus {
    Idle,
    Decoding,
    Ready(Arc<DecodedVideoClip>),
    Failed(String),
}

struct VideoPlayerInternal {
    owner_entity: EntityId,
    target_entity: EntityId,
    clip_path: Option<PathBuf>,
    cooked: Option<CookedVideo>,
    decoded: Option<Arc<DecodedVideoClip>>,
    worker_state: Arc<Mutex<WorkerStatus>>,
    state: VideoPlayerState,
    current_time: f64,
    current_frame: i64,
    presented_frame: i64,
    playback_speed: f32,
    is_looping: bool,
    play_when_prepared: bool,
    send_frame_ready_events: bool,
    pending_events: VideoEvents,
    last_error: Option<String>,
}

fn video_players_map() -> &'static Mutex<BTreeMap<u64, Arc<Mutex<VideoPlayerInternal>>>> {
    VIDEO_PLAYERS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

#[cfg(target_arch = "wasm32")]
pub fn ffmpeg_libraries_available() -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ffmpeg_libraries_available() -> bool {
    FfmpegApi::get().is_ok()
}

#[cfg(target_arch = "wasm32")]
pub fn decode_cooked_video(_cooked: &CookedVideo) -> Result<DecodedVideoClip, VideoDecodeError> {
    Err(VideoDecodeError::PlatformLimited(
        "FFmpeg dynamic decoding is not available on wasm32".to_string(),
    ))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn decode_cooked_video(cooked: &CookedVideo) -> Result<DecodedVideoClip, VideoDecodeError> {
    let api = FfmpegApi::get()?;
    let seq = VIDEO_TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let temp_path = std::env::temp_dir().join(format!(
        "unitas_video_decode_{}_{seq}.{}",
        std::process::id(),
        cooked.container.as_str()
    ));
    fs::write(&temp_path, &cooked.payload)
        .map_err(|err| VideoDecodeError::IoError(err.to_string()))?;
    let result = api.decode_container_file(&temp_path, cooked);
    let _ = fs::remove_file(&temp_path);
    result
}

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
#[derive(Clone, Copy)]
struct AvRational {
    num: c_int,
    den: c_int,
}

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
struct AvFormatContextPrefix {
    av_class: *const c_void,
    iformat: *const c_void,
    oformat: *const c_void,
    priv_data: *mut c_void,
    pb: *mut c_void,
    ctx_flags: c_int,
    nb_streams: c_uint,
    streams: *mut *mut AvStreamPrefix,
}

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
struct AvStreamPrefix {
    av_class: *const c_void,
    index: c_int,
    id: c_int,
    codecpar: *mut AvCodecParametersPrefix,
    priv_data: *mut c_void,
    time_base: AvRational,
}

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
struct AvCodecParametersPrefix {
    codec_type: c_int,
    codec_id: c_int,
    codec_tag: u32,
    extradata: *mut u8,
    extradata_size: c_int,
    coded_side_data: *mut c_void,
    nb_coded_side_data: c_int,
    format: c_int,
    bit_rate: i64,
    bits_per_coded_sample: c_int,
    bits_per_raw_sample: c_int,
    profile: c_int,
    level: c_int,
    width: c_int,
    height: c_int,
}

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
struct AvPacketPrefix {
    buf: *mut c_void,
    pts: i64,
    dts: i64,
    data: *mut u8,
    size: c_int,
    stream_index: c_int,
    flags: c_int,
}

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
struct AvFramePrefix {
    data: [*mut u8; 8],
    linesize: [c_int; 8],
    extended_data: *mut *mut u8,
    width: c_int,
    height: c_int,
    nb_samples: c_int,
    format: c_int,
}

#[cfg(not(target_arch = "wasm32"))]
type AvFormatOpenInputFn = unsafe extern "C" fn(
    *mut *mut AvFormatContextPrefix,
    *const c_char,
    *const c_void,
    *mut *mut c_void,
) -> c_int;
#[cfg(not(target_arch = "wasm32"))]
type AvFormatFindStreamInfoFn =
    unsafe extern "C" fn(*mut AvFormatContextPrefix, *mut *mut c_void) -> c_int;
#[cfg(not(target_arch = "wasm32"))]
type AvFindBestStreamFn = unsafe extern "C" fn(
    *mut AvFormatContextPrefix,
    c_int,
    c_int,
    c_int,
    *mut *const c_void,
    c_int,
) -> c_int;
#[cfg(not(target_arch = "wasm32"))]
type AvReadFrameFn = unsafe extern "C" fn(*mut AvFormatContextPrefix, *mut AvPacketPrefix) -> c_int;
#[cfg(not(target_arch = "wasm32"))]
type AvFormatCloseInputFn = unsafe extern "C" fn(*mut *mut AvFormatContextPrefix);
#[cfg(not(target_arch = "wasm32"))]
type AvCodecFindDecoderFn = unsafe extern "C" fn(c_int) -> *const c_void;
#[cfg(not(target_arch = "wasm32"))]
type AvCodecAllocContext3Fn = unsafe extern "C" fn(*const c_void) -> *mut c_void;
#[cfg(not(target_arch = "wasm32"))]
type AvCodecParametersToContextFn =
    unsafe extern "C" fn(*mut c_void, *const AvCodecParametersPrefix) -> c_int;
#[cfg(not(target_arch = "wasm32"))]
type AvCodecOpen2Fn = unsafe extern "C" fn(*mut c_void, *const c_void, *mut *mut c_void) -> c_int;
#[cfg(not(target_arch = "wasm32"))]
type AvCodecSendPacketFn = unsafe extern "C" fn(*mut c_void, *const AvPacketPrefix) -> c_int;
#[cfg(not(target_arch = "wasm32"))]
type AvCodecReceiveFrameFn = unsafe extern "C" fn(*mut c_void, *mut AvFramePrefix) -> c_int;
#[cfg(not(target_arch = "wasm32"))]
type AvCodecFreeContextFn = unsafe extern "C" fn(*mut *mut c_void);
#[cfg(not(target_arch = "wasm32"))]
type AvPacketAllocFn = unsafe extern "C" fn() -> *mut AvPacketPrefix;
#[cfg(not(target_arch = "wasm32"))]
type AvPacketUnrefFn = unsafe extern "C" fn(*mut AvPacketPrefix);
#[cfg(not(target_arch = "wasm32"))]
type AvPacketFreeFn = unsafe extern "C" fn(*mut *mut AvPacketPrefix);
#[cfg(not(target_arch = "wasm32"))]
type AvFrameAllocFn = unsafe extern "C" fn() -> *mut AvFramePrefix;
#[cfg(not(target_arch = "wasm32"))]
type AvFrameUnrefFn = unsafe extern "C" fn(*mut AvFramePrefix);
#[cfg(not(target_arch = "wasm32"))]
type AvFrameFreeFn = unsafe extern "C" fn(*mut *mut AvFramePrefix);
#[cfg(not(target_arch = "wasm32"))]
type SwsGetContextFn = unsafe extern "C" fn(
    c_int,
    c_int,
    c_int,
    c_int,
    c_int,
    c_int,
    c_int,
    *mut c_void,
    *mut c_void,
    *const c_double,
) -> *mut c_void;
#[cfg(not(target_arch = "wasm32"))]
type SwsScaleFn = unsafe extern "C" fn(
    *mut c_void,
    *const *const u8,
    *const c_int,
    c_int,
    c_int,
    *const *mut u8,
    *const c_int,
) -> c_int;
#[cfg(not(target_arch = "wasm32"))]
type SwsFreeContextFn = unsafe extern "C" fn(*mut c_void);

#[cfg(not(target_arch = "wasm32"))]
struct FfmpegApi {
    _avutil: Library,
    _swscale: Library,
    _avcodec: Library,
    _avformat: Library,
    avformat_open_input: AvFormatOpenInputFn,
    avformat_find_stream_info: AvFormatFindStreamInfoFn,
    av_find_best_stream: AvFindBestStreamFn,
    av_read_frame: AvReadFrameFn,
    avformat_close_input: AvFormatCloseInputFn,
    avcodec_find_decoder: AvCodecFindDecoderFn,
    avcodec_alloc_context3: AvCodecAllocContext3Fn,
    avcodec_parameters_to_context: AvCodecParametersToContextFn,
    avcodec_open2: AvCodecOpen2Fn,
    avcodec_send_packet: AvCodecSendPacketFn,
    avcodec_receive_frame: AvCodecReceiveFrameFn,
    avcodec_free_context: AvCodecFreeContextFn,
    av_packet_alloc: AvPacketAllocFn,
    av_packet_unref: AvPacketUnrefFn,
    av_packet_free: AvPacketFreeFn,
    av_frame_alloc: AvFrameAllocFn,
    av_frame_unref: AvFrameUnrefFn,
    av_frame_free: AvFrameFreeFn,
    sws_get_context: SwsGetContextFn,
    sws_scale: SwsScaleFn,
    sws_free_context: SwsFreeContextFn,
}

#[cfg(not(target_arch = "wasm32"))]
static FFMPEG_API: OnceLock<Result<FfmpegApi, VideoDecodeError>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
impl FfmpegApi {
    fn get() -> Result<&'static Self, VideoDecodeError> {
        match FFMPEG_API.get_or_init(Self::load) {
            Ok(api) => Ok(api),
            Err(err) => Err(err.clone()),
        }
    }

    fn load_libraries() -> Result<(Library, Library, Library, Library), VideoDecodeError> {
        let avutil = load_ffmpeg_library(&[
            "/opt/homebrew/lib/libavutil.dylib",
            "/opt/homebrew/lib/libavutil.60.dylib",
            "/usr/local/lib/libavutil.dylib",
            "libavutil.dylib",
            "libavutil.so.60",
            "libavutil.so.59",
            "libavutil.so.58",
            "libavutil.so",
            "avutil-60.dll",
            "avutil-59.dll",
            "avutil-58.dll",
        ])?;
        let swscale = load_ffmpeg_library(&[
            "/opt/homebrew/lib/libswscale.dylib",
            "/opt/homebrew/lib/libswscale.9.dylib",
            "/usr/local/lib/libswscale.dylib",
            "libswscale.dylib",
            "libswscale.so.9",
            "libswscale.so.8",
            "libswscale.so.7",
            "libswscale.so",
            "swscale-9.dll",
            "swscale-8.dll",
            "swscale-7.dll",
        ])?;
        let avcodec = load_ffmpeg_library(&[
            "/opt/homebrew/lib/libavcodec.dylib",
            "/opt/homebrew/lib/libavcodec.62.dylib",
            "/usr/local/lib/libavcodec.dylib",
            "libavcodec.dylib",
            "libavcodec.so.62",
            "libavcodec.so.61",
            "libavcodec.so.60",
            "libavcodec.so",
            "avcodec-62.dll",
            "avcodec-61.dll",
            "avcodec-60.dll",
        ])?;
        let avformat = load_ffmpeg_library(&[
            "/opt/homebrew/lib/libavformat.dylib",
            "/opt/homebrew/lib/libavformat.62.dylib",
            "/usr/local/lib/libavformat.dylib",
            "libavformat.dylib",
            "libavformat.so.62",
            "libavformat.so.61",
            "libavformat.so.60",
            "libavformat.so",
            "avformat-62.dll",
            "avformat-61.dll",
            "avformat-60.dll",
        ])?;
        Ok((avutil, swscale, avcodec, avformat))
    }

    fn load() -> Result<Self, VideoDecodeError> {
        let (avutil, swscale, avcodec, avformat) = Self::load_libraries()?;

        let avformat_open_input = load_symbol(&avformat, b"avformat_open_input\0")?;
        let avformat_find_stream_info = load_symbol(&avformat, b"avformat_find_stream_info\0")?;
        let av_find_best_stream = load_symbol(&avformat, b"av_find_best_stream\0")?;
        let av_read_frame = load_symbol(&avformat, b"av_read_frame\0")?;
        let avformat_close_input = load_symbol(&avformat, b"avformat_close_input\0")?;

        let avcodec_find_decoder = load_symbol(&avcodec, b"avcodec_find_decoder\0")?;
        let avcodec_alloc_context3 = load_symbol(&avcodec, b"avcodec_alloc_context3\0")?;
        let avcodec_parameters_to_context =
            load_symbol(&avcodec, b"avcodec_parameters_to_context\0")?;
        let avcodec_open2 = load_symbol(&avcodec, b"avcodec_open2\0")?;
        let avcodec_send_packet = load_symbol(&avcodec, b"avcodec_send_packet\0")?;
        let avcodec_receive_frame = load_symbol(&avcodec, b"avcodec_receive_frame\0")?;
        let avcodec_free_context = load_symbol(&avcodec, b"avcodec_free_context\0")?;
        let av_packet_alloc = load_symbol(&avcodec, b"av_packet_alloc\0")?;
        let av_packet_unref = load_symbol(&avcodec, b"av_packet_unref\0")?;
        let av_packet_free = load_symbol(&avcodec, b"av_packet_free\0")?;

        let av_frame_alloc = load_symbol(&avutil, b"av_frame_alloc\0")?;
        let av_frame_unref = load_symbol(&avutil, b"av_frame_unref\0")?;
        let av_frame_free = load_symbol(&avutil, b"av_frame_free\0")?;

        let sws_get_context = load_symbol(&swscale, b"sws_getContext\0")?;
        let sws_scale = load_symbol(&swscale, b"sws_scale\0")?;
        let sws_free_context = load_symbol(&swscale, b"sws_freeContext\0")?;

        Ok(Self {
            _avutil: avutil,
            _swscale: swscale,
            _avcodec: avcodec,
            _avformat: avformat,
            avformat_open_input,
            avformat_find_stream_info,
            av_find_best_stream,
            av_read_frame,
            avformat_close_input,
            avcodec_find_decoder,
            avcodec_alloc_context3,
            avcodec_parameters_to_context,
            avcodec_open2,
            avcodec_send_packet,
            avcodec_receive_frame,
            avcodec_free_context,
            av_packet_alloc,
            av_packet_unref,
            av_packet_free,
            av_frame_alloc,
            av_frame_unref,
            av_frame_free,
            sws_get_context,
            sws_scale,
            sws_free_context,
        })
    }

    fn decode_container_file(
        &self,
        path: &Path,
        cooked: &CookedVideo,
    ) -> Result<DecodedVideoClip, VideoDecodeError> {
        let path_str = path.to_string_lossy();
        let c_url = CString::new(path_str.as_bytes())
            .map_err(|_| VideoDecodeError::IoError("invalid NUL in video path".to_string()))?;
        let mut fmt_ctx: *mut AvFormatContextPrefix = std::ptr::null_mut();
        // SAFETY: fmt_ctx is a valid local pointer and c_url is a NUL-terminated path string.
        let open_rc = unsafe {
            (self.avformat_open_input)(
                std::ptr::from_mut(&mut fmt_ctx),
                c_url.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
            )
        };
        if open_rc < 0 || fmt_ctx.is_null() {
            return Err(VideoDecodeError::CorruptStream(format!(
                "avformat_open_input failed with code {open_rc} for container {}",
                cooked.container
            )));
        }
        let decode_res = self.decode_opened_format(fmt_ctx, cooked);
        // SAFETY: fmt_ctx was allocated by avformat_open_input and is closed once here.
        unsafe { (self.avformat_close_input)(std::ptr::from_mut(&mut fmt_ctx)) };
        decode_res
    }

    fn decode_opened_format(
        &self,
        fmt_ctx: *mut AvFormatContextPrefix,
        cooked: &CookedVideo,
    ) -> Result<DecodedVideoClip, VideoDecodeError> {
        // SAFETY: fmt_ctx is an open AVFormatContext returned by avformat_open_input.
        let info_rc = unsafe { (self.avformat_find_stream_info)(fmt_ctx, std::ptr::null_mut()) };
        if info_rc < 0 {
            return Err(VideoDecodeError::CorruptStream(format!(
                "avformat_find_stream_info failed with code {info_rc}"
            )));
        }
        let mut decoder: *const c_void = std::ptr::null();
        // SAFETY: fmt_ctx is valid and decoder is a valid local out-pointer.
        let stream_index = unsafe {
            (self.av_find_best_stream)(
                fmt_ctx,
                AVMEDIA_TYPE_VIDEO,
                -1,
                -1,
                std::ptr::from_mut(&mut decoder),
                0,
            )
        };
        if stream_index < 0 {
            return Err(VideoDecodeError::CorruptStream(format!(
                "no decodable video stream found (code {stream_index})"
            )));
        }
        let codecpar = self.extract_stream_codecpar(fmt_ctx, stream_index)?;
        if decoder.is_null() {
            // SAFETY: codecpar points to a valid AVCodecParameters owned by the stream.
            let codec_id = unsafe { (*codecpar).codec_id };
            // SAFETY: avcodec_find_decoder is safe to call with any integer codec ID.
            decoder = unsafe { (self.avcodec_find_decoder)(codec_id) };
        }
        if decoder.is_null() {
            return Err(VideoDecodeError::UnsupportedCodec(format!(
                "no FFmpeg decoder found for codec {}",
                cooked.codec_name()
            )));
        }
        // SAFETY: decoder is a valid AVCodec pointer returned by libavformat/libavcodec.
        let mut codec_ctx = unsafe { (self.avcodec_alloc_context3)(decoder) };
        if codec_ctx.is_null() {
            return Err(VideoDecodeError::LibraryUnavailable(
                "avcodec_alloc_context3 returned null".to_string(),
            ));
        }
        let result = self.run_codec_decode_loop(fmt_ctx, codec_ctx, codecpar, stream_index, cooked);
        // SAFETY: codec_ctx was allocated by avcodec_alloc_context3 and is freed once here.
        unsafe { (self.avcodec_free_context)(std::ptr::from_mut(&mut codec_ctx)) };
        result
    }

    fn extract_stream_codecpar(
        &self,
        fmt_ctx: *mut AvFormatContextPrefix,
        stream_index: c_int,
    ) -> Result<*const AvCodecParametersPrefix, VideoDecodeError> {
        let _ = self;
        // SAFETY: fmt_ctx is a valid pointer to AVFormatContextPrefix.
        let nb_streams = unsafe { (*fmt_ctx).nb_streams } as usize;
        // SAFETY: fmt_ctx is a valid pointer to AVFormatContextPrefix.
        let streams_ptr = unsafe { (*fmt_ctx).streams };
        let idx = usize::try_from(stream_index).unwrap_or(usize::MAX);
        if idx >= nb_streams || streams_ptr.is_null() {
            return Err(VideoDecodeError::CorruptStream(
                "video stream index out of bounds".to_string(),
            ));
        }
        // SAFETY: idx < nb_streams and streams_ptr is the libavformat stream pointer array.
        let stream_slot = unsafe { streams_ptr.add(idx) };
        // SAFETY: stream_slot points to a valid entry in the libavformat stream pointer array.
        let stream_ptr = unsafe { *stream_slot };
        if stream_ptr.is_null() {
            return Err(VideoDecodeError::CorruptStream(
                "stream pointer is null".to_string(),
            ));
        }
        // SAFETY: stream_ptr is a valid AVStream pointer from fmt_ctx->streams.
        let codecpar = unsafe { (*stream_ptr).codecpar };
        if codecpar.is_null() {
            return Err(VideoDecodeError::CorruptStream(
                "stream codecpar is null".to_string(),
            ));
        }
        Ok(codecpar.cast_const())
    }

    fn open_codec_context(
        &self,
        codec_ctx: *mut c_void,
        codecpar: *const AvCodecParametersPrefix,
    ) -> Result<(), VideoDecodeError> {
        // SAFETY: codec_ctx and codecpar are valid pointers from libavcodec/libavformat.
        let par_rc = unsafe { (self.avcodec_parameters_to_context)(codec_ctx, codecpar) };
        if par_rc < 0 {
            return Err(VideoDecodeError::CorruptStream(format!(
                "avcodec_parameters_to_context failed with code {par_rc}"
            )));
        }
        // SAFETY: codec_ctx was configured with parameters and is opened with its bound codec.
        let open_rc =
            unsafe { (self.avcodec_open2)(codec_ctx, std::ptr::null(), std::ptr::null_mut()) };
        if open_rc < 0 {
            return Err(VideoDecodeError::UnsupportedCodec(format!(
                "avcodec_open2 failed with code {open_rc}"
            )));
        }
        Ok(())
    }

    fn run_codec_decode_loop(
        &self,
        fmt_ctx: *mut AvFormatContextPrefix,
        codec_ctx: *mut c_void,
        codecpar: *const AvCodecParametersPrefix,
        stream_index: c_int,
        cooked: &CookedVideo,
    ) -> Result<DecodedVideoClip, VideoDecodeError> {
        self.open_codec_context(codec_ctx, codecpar)?;
        // SAFETY: av_packet_alloc takes no arguments and returns a new AVPacket or null.
        let mut packet = unsafe { (self.av_packet_alloc)() };
        if packet.is_null() {
            return Err(VideoDecodeError::LibraryUnavailable(
                "av_packet_alloc returned null".to_string(),
            ));
        }
        // SAFETY: av_frame_alloc takes no arguments and returns a new AVFrame or null.
        let mut frame = unsafe { (self.av_frame_alloc)() };
        if frame.is_null() {
            // SAFETY: packet was allocated by av_packet_alloc.
            unsafe { (self.av_packet_free)(std::ptr::from_mut(&mut packet)) };
            return Err(VideoDecodeError::LibraryUnavailable(
                "av_frame_alloc returned null".to_string(),
            ));
        }
        let frames_res =
            self.read_all_packets_and_frames(fmt_ctx, codec_ctx, packet, frame, stream_index);
        // SAFETY: frame was allocated by av_frame_alloc.
        unsafe { (self.av_frame_free)(std::ptr::from_mut(&mut frame)) };
        // SAFETY: packet was allocated by av_packet_alloc.
        unsafe { (self.av_packet_free)(std::ptr::from_mut(&mut packet)) };
        let (frames_rgba, decoded_w, decoded_h) = frames_res?;
        if frames_rgba.is_empty() {
            return Err(VideoDecodeError::CorruptStream(
                "video stream produced zero decoded frames".to_string(),
            ));
        }
        Ok(DecodedVideoClip {
            width: if decoded_w > 0 {
                decoded_w
            } else {
                cooked.width
            },
            height: if decoded_h > 0 {
                decoded_h
            } else {
                cooked.height
            },
            fps_num: cooked.fps_num.max(1),
            fps_den: cooked.fps_den.max(1),
            duration_micros: cooked.duration_micros,
            frames_rgba,
        })
    }

    fn read_all_packets_and_frames(
        &self,
        fmt_ctx: *mut AvFormatContextPrefix,
        codec_ctx: *mut c_void,
        packet: *mut AvPacketPrefix,
        frame: *mut AvFramePrefix,
        stream_index: c_int,
    ) -> Result<(Vec<Vec<u8>>, u32, u32), VideoDecodeError> {
        let mut frames = Vec::new();
        let mut out_w = 0u32;
        let mut out_h = 0u32;
        loop {
            // SAFETY: fmt_ctx and packet are valid allocated FFmpeg structs.
            let read_rc = unsafe { (self.av_read_frame)(fmt_ctx, packet) };
            if read_rc < 0 {
                break;
            }
            // SAFETY: packet was populated by av_read_frame.
            let pkt_stream = unsafe { (*packet).stream_index };
            if pkt_stream == stream_index {
                // SAFETY: codec_ctx is open and packet contains a demuxed packet.
                let send_rc = unsafe { (self.avcodec_send_packet)(codec_ctx, packet.cast_const()) };
                // SAFETY: packet references are released before the next read.
                unsafe { (self.av_packet_unref)(packet) };
                if send_rc < 0 {
                    return Err(VideoDecodeError::CorruptStream(format!(
                        "avcodec_send_packet failed with code {send_rc}"
                    )));
                }
                self.drain_decoded_frames(codec_ctx, frame, &mut frames, &mut out_w, &mut out_h)?;
            } else {
                // SAFETY: packet was populated by av_read_frame and must be unreferenced.
                unsafe { (self.av_packet_unref)(packet) };
            }
        }
        // SAFETY: sending a null packet flushes delayed frames from the decoder.
        let _ = unsafe { (self.avcodec_send_packet)(codec_ctx, std::ptr::null()) };
        self.drain_decoded_frames(codec_ctx, frame, &mut frames, &mut out_w, &mut out_h)?;
        Ok((frames, out_w, out_h))
    }

    fn drain_decoded_frames(
        &self,
        codec_ctx: *mut c_void,
        frame: *mut AvFramePrefix,
        frames: &mut Vec<Vec<u8>>,
        out_w: &mut u32,
        out_h: &mut u32,
    ) -> Result<(), VideoDecodeError> {
        loop {
            // SAFETY: codec_ctx is open and frame is an allocated AVFrame.
            let recv_rc = unsafe { (self.avcodec_receive_frame)(codec_ctx, frame) };
            if recv_rc < 0 {
                break;
            }
            let (rgba, width, height) = self.convert_frame_to_rgba(frame)?;
            *out_w = width;
            *out_h = height;
            frames.push(rgba);
            // SAFETY: frame was populated by avcodec_receive_frame and is unreferenced after copy.
            unsafe { (self.av_frame_unref)(frame) };
        }
        Ok(())
    }

    fn read_frame_header(
        frame: *mut AvFramePrefix,
    ) -> Result<(c_int, c_int, c_int), VideoDecodeError> {
        // SAFETY: frame points to a decoded AVFrame from avcodec_receive_frame.
        let width = unsafe { (*frame).width };
        // SAFETY: frame points to a decoded AVFrame from avcodec_receive_frame.
        let height = unsafe { (*frame).height };
        // SAFETY: frame points to a decoded AVFrame from avcodec_receive_frame.
        let pix_fmt = unsafe { (*frame).format };
        if width <= 0 || height <= 0 || pix_fmt < 0 {
            return Err(VideoDecodeError::CorruptStream(format!(
                "invalid decoded frame dimensions {width}x{height} format {pix_fmt}"
            )));
        }
        Ok((width, height, pix_fmt))
    }

    fn convert_frame_to_rgba(
        &self,
        frame: *mut AvFramePrefix,
    ) -> Result<(Vec<u8>, u32, u32), VideoDecodeError> {
        let (width, height, pix_fmt) = Self::read_frame_header(frame)?;
        // SAFETY: sws_getContext allocates a scaler context for valid dimensions and pixel formats.
        let sws = unsafe {
            (self.sws_get_context)(
                width,
                height,
                pix_fmt,
                width,
                height,
                AV_PIX_FMT_RGBA,
                SWS_BILINEAR,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        if sws.is_null() {
            return Err(VideoDecodeError::UnsupportedCodec(
                "sws_getContext returned null".to_string(),
            ));
        }
        let w_usize = usize::try_from(width).unwrap_or(0);
        let h_usize = usize::try_from(height).unwrap_or(0);
        let mut rgba = vec![0u8; w_usize * h_usize * 4];
        let dst_data: [*mut u8; 4] = [
            rgba.as_mut_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ];
        let dst_linesize: [c_int; 4] = [width * 4, 0, 0, 0];
        // SAFETY: frame is valid; data array is at offset 0 of AVFrame.
        let src_data = unsafe { (*frame).data };
        // SAFETY: frame is valid; linesize array is at offset 64 of AVFrame.
        let src_linesize = unsafe { (*frame).linesize };
        let src_data_const: [*const u8; 8] = src_data.map(|ptr| ptr.cast_const());
        // SAFETY: sws is valid, src_data_const comes from the decoded AVFrame, and dst_data points to rgba.
        let scaled_h = unsafe {
            (self.sws_scale)(
                sws,
                src_data_const.as_ptr(),
                src_linesize.as_ptr(),
                0,
                height,
                dst_data.as_ptr(),
                dst_linesize.as_ptr(),
            )
        };
        // SAFETY: sws was allocated by sws_getContext and is freed once here.
        unsafe { (self.sws_free_context)(sws) };
        if scaled_h <= 0 {
            return Err(VideoDecodeError::CorruptStream(
                "sws_scale failed to convert frame to RGBA".to_string(),
            ));
        }
        Ok((rgba, w_usize as u32, h_usize as u32))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_ffmpeg_library(candidates: &[&str]) -> Result<Library, VideoDecodeError> {
    let mut errors = Vec::new();
    for candidate in candidates {
        // SAFETY: loading one of the standard FFmpeg dynamic libraries by candidate path/soname.
        let opened = unsafe { Library::new(candidate) };
        match opened {
            Ok(lib) => return Ok(lib),
            Err(err) => errors.push(format!("{candidate}: {err}")),
        }
    }
    Err(VideoDecodeError::LibraryUnavailable(errors.join("; ")))
}

#[cfg(not(target_arch = "wasm32"))]
fn load_symbol<T: Copy>(library: &Library, symbol: &[u8]) -> Result<T, VideoDecodeError> {
    // SAFETY: symbol is a NUL-terminated ASCII name matching the C signature T in the loaded FFmpeg library.
    let resolved = unsafe { library.get::<T>(symbol) };
    let sym = resolved.map_err(|err| {
        VideoDecodeError::LibraryUnavailable(format!(
            "missing symbol {}: {err}",
            String::from_utf8_lossy(symbol).trim_end_matches('\0')
        ))
    })?;
    Ok(*sym)
}

impl Runtime {
    pub fn create_video_player(&mut self, entity: EntityId) -> u64 {
        let player_id = VIDEO_PLAYER_ID_SEQ.fetch_add(1, Ordering::Relaxed);
        let internal = VideoPlayerInternal {
            owner_entity: entity,
            target_entity: entity,
            clip_path: None,
            cooked: None,
            decoded: None,
            worker_state: Arc::new(Mutex::new(WorkerStatus::Idle)),
            state: VideoPlayerState::Idle,
            current_time: 0.0,
            current_frame: -1,
            presented_frame: -1,
            playback_speed: 1.0,
            is_looping: false,
            play_when_prepared: false,
            send_frame_ready_events: false,
            pending_events: VideoEvents::default(),
            last_error: None,
        };
        if let Ok(mut map) = video_players_map().lock() {
            map.insert(player_id, Arc::new(Mutex::new(internal)));
        }
        player_id
    }

    pub fn destroy_video_player(&mut self, player_id: u64) -> bool {
        let Ok(mut map) = video_players_map().lock() else {
            return false;
        };
        map.remove(&player_id).is_some()
    }

    pub fn load_video_clip(&mut self, player_id: u64, path: &Path) -> bool {
        let Some(player_arc) = get_player_arc(player_id) else {
            return false;
        };
        let cooked = load_or_cook_video(path);
        let Ok(mut player) = player_arc.lock() else {
            return false;
        };
        player.clip_path = Some(path.to_path_buf());
        player.decoded = None;
        player.current_time = 0.0;
        player.current_frame = -1;
        player.presented_frame = -1;
        if let Some(cooked_clip) = cooked {
            player.cooked = Some(cooked_clip);
            player.state = VideoPlayerState::Idle;
            player.last_error = None;
            if let Ok(mut ws) = player.worker_state.lock() {
                *ws = WorkerStatus::Idle;
            }
            true
        } else {
            let msg = format!("failed to load or cook video clip '{}'", path.display());
            player.cooked = None;
            player.state = VideoPlayerState::Failed;
            player.pending_events.error_received = true;
            player.last_error = Some(msg.clone());
            self.record_video_diagnostic(player.owner_entity, msg);
            false
        }
    }

    pub fn prepare_video_player(&mut self, player_id: u64) -> bool {
        let Some(player_arc) = get_player_arc(player_id) else {
            return false;
        };
        let Ok(mut player) = player_arc.lock() else {
            return false;
        };
        if player.decoded.is_some() {
            player.pending_events.prepare_completed = true;
            return true;
        }
        let Some(cooked) = player.cooked.clone() else {
            let msg = "cannot prepare VideoPlayer without a valid video clip".to_string();
            player.state = VideoPlayerState::Failed;
            player.pending_events.error_received = true;
            player.last_error = Some(msg.clone());
            self.record_video_diagnostic(player.owner_entity, msg);
            return false;
        };
        player.state = VideoPlayerState::Preparing;
        let worker_state = Arc::clone(&player.worker_state);
        if let Ok(mut ws) = worker_state.lock() {
            *ws = WorkerStatus::Decoding;
        }
        std::thread::spawn(move || {
            let decoded = decode_cooked_video(&cooked);
            if let Ok(mut ws) = worker_state.lock() {
                match decoded {
                    Ok(clip) => *ws = WorkerStatus::Ready(Arc::new(clip)),
                    Err(err) => *ws = WorkerStatus::Failed(err.to_string()),
                }
            }
        });
        true
    }

    pub fn wait_video_player_prepared(&mut self, player_id: u64, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let _ = self.advance_video_player(player_id, 0.0);
            if let Some(snap) = self.video_player_snapshot(player_id) {
                if snap.is_prepared {
                    return true;
                }
                if snap.last_error.is_some() {
                    return false;
                }
            } else {
                return false;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    pub fn play_video_player(&mut self, player_id: u64) -> bool {
        let Some(player_arc) = get_player_arc(player_id) else {
            return false;
        };
        let needs_prepare = {
            let Ok(mut player) = player_arc.lock() else {
                return false;
            };
            sync_worker_into_player(&mut player);
            if player.decoded.is_some() {
                player.state = VideoPlayerState::Playing;
                player.pending_events.started = true;
                if player.current_frame < 0 {
                    player.current_frame = 0;
                    player.current_time = 0.0;
                }
                false
            } else {
                player.play_when_prepared = true;
                !matches!(player.state, VideoPlayerState::Preparing)
            }
        };
        if needs_prepare && !self.prepare_video_player(player_id) {
            return false;
        }
        self.present_current_video_frame(player_id);
        true
    }

    pub fn pause_video_player(&mut self, player_id: u64) -> bool {
        let Some(player_arc) = get_player_arc(player_id) else {
            return false;
        };
        let Ok(mut player) = player_arc.lock() else {
            return false;
        };
        player.play_when_prepared = false;
        if player.state == VideoPlayerState::Playing {
            player.state = VideoPlayerState::Paused;
        }
        true
    }

    pub fn stop_video_player(&mut self, player_id: u64) -> bool {
        let Some(player_arc) = get_player_arc(player_id) else {
            return false;
        };
        let Ok(mut player) = player_arc.lock() else {
            return false;
        };
        player.play_when_prepared = false;
        player.state = VideoPlayerState::Stopped;
        player.decoded = None;
        player.current_time = 0.0;
        player.current_frame = -1;
        player.presented_frame = -1;
        if let Ok(mut ws) = player.worker_state.lock() {
            *ws = WorkerStatus::Idle;
        }
        true
    }

    pub fn step_forward_video_player(&mut self, player_id: u64) -> bool {
        let Some(player_arc) = get_player_arc(player_id) else {
            return false;
        };
        {
            let Ok(mut player) = player_arc.lock() else {
                return false;
            };
            sync_worker_into_player(&mut player);
            let Some(decoded) = player.decoded.as_ref() else {
                return false;
            };
            let total = decoded.frames_rgba.len() as i64;
            if total <= 0 {
                return false;
            }
            let rate = decoded.frame_rate().max(1.0);
            let next_frame = (player.current_frame + 1).clamp(0, total - 1);
            player.current_frame = next_frame;
            player.current_time = next_frame as f64 / rate;
            if player.state == VideoPlayerState::Playing {
                player.state = VideoPlayerState::Paused;
            }
            if player.send_frame_ready_events {
                player.pending_events.frame_ready = true;
            }
        }
        self.present_current_video_frame(player_id);
        true
    }

    pub fn set_video_player_time(&mut self, player_id: u64, seconds: f64) -> bool {
        if !seconds.is_finite() || seconds < 0.0 {
            return false;
        }
        let Some(player_arc) = get_player_arc(player_id) else {
            return false;
        };
        {
            let Ok(mut player) = player_arc.lock() else {
                return false;
            };
            sync_worker_into_player(&mut player);
            player.current_time = seconds;
            if let Some(decoded) = player.decoded.as_ref() {
                let total = decoded.frames_rgba.len() as i64;
                let rate = decoded.frame_rate().max(1.0);
                if total > 0 {
                    let idx = ((seconds * rate).floor() as i64).clamp(0, total - 1);
                    player.current_frame = idx;
                }
            }
            player.pending_events.seek_completed = true;
            if player.send_frame_ready_events {
                player.pending_events.frame_ready = true;
            }
        }
        self.present_current_video_frame(player_id);
        true
    }

    pub fn set_video_player_frame(&mut self, player_id: u64, frame_index: i64) -> bool {
        if frame_index < 0 {
            return false;
        }
        let Some(player_arc) = get_player_arc(player_id) else {
            return false;
        };
        {
            let Ok(mut player) = player_arc.lock() else {
                return false;
            };
            sync_worker_into_player(&mut player);
            let frame_info = player.decoded.as_ref().map(|decoded| {
                (
                    decoded.frames_rgba.len() as i64,
                    decoded.frame_rate().max(1.0),
                )
            });
            if let Some((total, rate)) = frame_info {
                if total == 0 {
                    return false;
                }
                let clamped = frame_index.clamp(0, total - 1);
                player.current_frame = clamped;
                player.current_time = clamped as f64 / rate;
            } else {
                player.current_frame = frame_index;
            }
            player.pending_events.seek_completed = true;
            if player.send_frame_ready_events {
                player.pending_events.frame_ready = true;
            }
        }
        self.present_current_video_frame(player_id);
        true
    }

    pub fn set_video_player_looping(&mut self, player_id: u64, looping: bool) -> bool {
        let Some(player_arc) = get_player_arc(player_id) else {
            return false;
        };
        let Ok(mut player) = player_arc.lock() else {
            return false;
        };
        player.is_looping = looping;
        true
    }

    pub fn set_video_player_playback_speed(&mut self, player_id: u64, speed: f32) -> bool {
        if !speed.is_finite() || speed < 0.0 {
            return false;
        }
        let Some(player_arc) = get_player_arc(player_id) else {
            return false;
        };
        let Ok(mut player) = player_arc.lock() else {
            return false;
        };
        player.playback_speed = speed;
        true
    }

    pub fn set_video_player_send_frame_ready_events(
        &mut self,
        player_id: u64,
        enabled: bool,
    ) -> bool {
        let Some(player_arc) = get_player_arc(player_id) else {
            return false;
        };
        let Ok(mut player) = player_arc.lock() else {
            return false;
        };
        player.send_frame_ready_events = enabled;
        true
    }

    pub fn set_video_player_target_entity(&mut self, player_id: u64, target: EntityId) -> bool {
        let Some(player_arc) = get_player_arc(player_id) else {
            return false;
        };
        {
            let Ok(mut player) = player_arc.lock() else {
                return false;
            };
            player.target_entity = target;
            player.presented_frame = -1;
        }
        self.present_current_video_frame(player_id);
        true
    }

    pub fn advance_video_player(&mut self, player_id: u64, delta_seconds: f64) -> bool {
        if !delta_seconds.is_finite() || delta_seconds < 0.0 {
            return false;
        }
        let Some(player_arc) = get_player_arc(player_id) else {
            return false;
        };
        let mut diag_to_push: Option<(EntityId, String)> = None;
        {
            let Ok(mut player) = player_arc.lock() else {
                return false;
            };
            if let Some(err_msg) = sync_worker_into_player(&mut player) {
                diag_to_push = Some((player.owner_entity, err_msg));
            }
            if player.state == VideoPlayerState::Playing
                && let Some(decoded) = player.decoded.clone()
            {
                advance_playing_state(&mut player, &decoded, delta_seconds);
            }
        }
        if let Some((entity, msg)) = diag_to_push {
            self.record_video_diagnostic(entity, msg);
        }
        self.present_current_video_frame(player_id);
        true
    }

    pub fn video_player_snapshot(&self, player_id: u64) -> Option<VideoPlayerSnapshot> {
        let player_arc = get_player_arc(player_id)?;
        let mut player = player_arc.lock().ok()?;
        let _ = sync_worker_into_player(&mut player);
        Some(snapshot_from_player(&player))
    }

    pub fn take_video_player_snapshot(&mut self, player_id: u64) -> Option<VideoPlayerSnapshot> {
        let player_arc = get_player_arc(player_id)?;
        let mut player = player_arc.lock().ok()?;
        if let Some(err_msg) = sync_worker_into_player(&mut player) {
            self.record_video_diagnostic(player.owner_entity, err_msg);
        }
        let snap = snapshot_from_player(&player);
        player.pending_events = VideoEvents::default();
        Some(snap)
    }

    fn record_video_diagnostic(&mut self, entity: EntityId, message: String) {
        if let Some(index) = self.active_scene
            && let Some(scene) = self.scenes.get_mut(index)
        {
            if !scene
                .diagnostics
                .iter()
                .any(|(id, text)| *id == entity && *text == message)
            {
                scene.diagnostics.push((entity, message));
            }
            return;
        }
        if !self
            .extra_diagnostics
            .iter()
            .any(|(id, text)| *id == entity && *text == message)
        {
            self.extra_diagnostics.push((entity, message));
        }
    }

    pub fn video_player_current_frame_rgba(&self, player_id: u64) -> Option<Vec<u8>> {
        let player_arc = get_player_arc(player_id)?;
        let mut player = player_arc.lock().ok()?;
        let _ = sync_worker_into_player(&mut player);
        let decoded = player.decoded.as_ref()?;
        let idx = usize::try_from(player.current_frame.max(0)).ok()?;
        decoded.frames_rgba.get(idx).cloned()
    }

    fn present_current_video_frame(&mut self, player_id: u64) {
        let Some(player_arc) = get_player_arc(player_id) else {
            return;
        };
        let frame_to_present = {
            let Ok(mut player) = player_arc.lock() else {
                return;
            };
            let Some(decoded) = player.decoded.as_ref() else {
                return;
            };
            if player.target_entity.as_u64() == 0 || player.current_frame < 0 {
                return;
            }
            if player.presented_frame == player.current_frame {
                return;
            }
            let Ok(idx) = usize::try_from(player.current_frame) else {
                return;
            };
            let Some(rgba) = decoded.frames_rgba.get(idx).cloned() else {
                return;
            };
            let width = decoded.width;
            let height = decoded.height;
            player.presented_frame = player.current_frame;
            Some((player.target_entity, width, height, rgba))
        };
        let Some((target_entity, width, height, rgba)) = frame_to_present else {
            return;
        };
        let _ = self.set_runtime_sprite_pixels(target_entity, width, height, rgba);
    }
}

fn get_player_arc(player_id: u64) -> Option<Arc<Mutex<VideoPlayerInternal>>> {
    let map = video_players_map().lock().ok()?;
    map.get(&player_id).cloned()
}

pub(crate) fn create_video_player_by_id(entity: EntityId) -> u64 {
    let player_id = VIDEO_PLAYER_ID_SEQ.fetch_add(1, Ordering::Relaxed);
    let internal = VideoPlayerInternal {
        owner_entity: entity,
        target_entity: entity,
        clip_path: None,
        cooked: None,
        decoded: None,
        worker_state: Arc::new(Mutex::new(WorkerStatus::Idle)),
        state: VideoPlayerState::Idle,
        current_time: 0.0,
        current_frame: -1,
        presented_frame: -1,
        playback_speed: 1.0,
        is_looping: false,
        play_when_prepared: false,
        send_frame_ready_events: false,
        pending_events: VideoEvents::default(),
        last_error: None,
    };
    if let Ok(mut map) = video_players_map().lock() {
        map.insert(player_id, Arc::new(Mutex::new(internal)));
    }
    player_id
}

pub(crate) fn destroy_video_player_by_id(player_id: u64) -> bool {
    let Ok(mut map) = video_players_map().lock() else {
        return false;
    };
    map.remove(&player_id).is_some()
}

pub(crate) fn load_video_clip_by_id(player_id: u64, path: &Path) -> bool {
    let Some(player_arc) = get_player_arc(player_id) else {
        return false;
    };
    let cooked = load_or_cook_video(path);
    let Ok(mut player) = player_arc.lock() else {
        return false;
    };
    player.clip_path = Some(path.to_path_buf());
    player.decoded = None;
    player.current_time = 0.0;
    player.current_frame = -1;
    player.presented_frame = -1;
    if let Some(cooked_clip) = cooked {
        player.cooked = Some(cooked_clip);
        player.state = VideoPlayerState::Idle;
        player.last_error = None;
        if let Ok(mut ws) = player.worker_state.lock() {
            *ws = WorkerStatus::Idle;
        }
        true
    } else {
        let msg = format!("failed to load or cook video clip '{}'", path.display());
        player.cooked = None;
        player.state = VideoPlayerState::Failed;
        player.pending_events.error_received = true;
        player.last_error = Some(msg);
        false
    }
}

pub(crate) fn prepare_video_player_by_id(player_id: u64) -> bool {
    let Some(player_arc) = get_player_arc(player_id) else {
        return false;
    };
    let Ok(mut player) = player_arc.lock() else {
        return false;
    };
    if player.decoded.is_some() {
        player.pending_events.prepare_completed = true;
        return true;
    }
    let Some(cooked) = player.cooked.clone() else {
        let msg = "cannot prepare VideoPlayer without a valid video clip".to_string();
        player.state = VideoPlayerState::Failed;
        player.pending_events.error_received = true;
        player.last_error = Some(msg);
        return false;
    };
    player.state = VideoPlayerState::Preparing;
    let worker_state = Arc::clone(&player.worker_state);
    if let Ok(mut ws) = worker_state.lock() {
        *ws = WorkerStatus::Decoding;
    }
    std::thread::spawn(move || {
        let decoded = decode_cooked_video(&cooked);
        if let Ok(mut ws) = worker_state.lock() {
            match decoded {
                Ok(clip) => *ws = WorkerStatus::Ready(Arc::new(clip)),
                Err(err) => *ws = WorkerStatus::Failed(err.to_string()),
            }
        }
    });
    true
}

pub(crate) fn wait_video_player_prepared_by_id(player_id: u64, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        let _ = advance_video_player_by_id(player_id, 0.0);
        if let Some(player_arc) = get_player_arc(player_id)
            && let Ok(mut player) = player_arc.lock()
        {
            let _ = sync_worker_into_player(&mut player);
            if player.decoded.is_some() {
                return true;
            }
            if player.last_error.is_some() {
                return false;
            }
        } else {
            return false;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

pub(crate) fn play_video_player_by_id(player_id: u64) -> bool {
    let Some(player_arc) = get_player_arc(player_id) else {
        return false;
    };
    let needs_prepare = {
        let Ok(mut player) = player_arc.lock() else {
            return false;
        };
        let _ = sync_worker_into_player(&mut player);
        if player.decoded.is_some() {
            player.state = VideoPlayerState::Playing;
            player.pending_events.started = true;
            if player.current_frame < 0 {
                player.current_frame = 0;
                player.current_time = 0.0;
            }
            false
        } else {
            player.play_when_prepared = true;
            !matches!(player.state, VideoPlayerState::Preparing)
        }
    };
    if needs_prepare && !prepare_video_player_by_id(player_id) {
        return false;
    }
    true
}

pub(crate) fn pause_video_player_by_id(player_id: u64) -> bool {
    let Some(player_arc) = get_player_arc(player_id) else {
        return false;
    };
    let Ok(mut player) = player_arc.lock() else {
        return false;
    };
    player.play_when_prepared = false;
    if player.state == VideoPlayerState::Playing {
        player.state = VideoPlayerState::Paused;
    }
    true
}

pub(crate) fn stop_video_player_by_id(player_id: u64) -> bool {
    let Some(player_arc) = get_player_arc(player_id) else {
        return false;
    };
    let Ok(mut player) = player_arc.lock() else {
        return false;
    };
    player.play_when_prepared = false;
    player.state = VideoPlayerState::Stopped;
    player.decoded = None;
    player.current_time = 0.0;
    player.current_frame = -1;
    player.presented_frame = -1;
    if let Ok(mut ws) = player.worker_state.lock() {
        *ws = WorkerStatus::Idle;
    }
    true
}

pub(crate) fn step_forward_video_player_by_id(player_id: u64) -> bool {
    let Some(player_arc) = get_player_arc(player_id) else {
        return false;
    };
    let Ok(mut player) = player_arc.lock() else {
        return false;
    };
    let _ = sync_worker_into_player(&mut player);
    let Some(decoded) = player.decoded.as_ref() else {
        return false;
    };
    let total = decoded.frames_rgba.len() as i64;
    if total <= 0 {
        return false;
    }
    let rate = decoded.frame_rate().max(1.0);
    let next_frame = (player.current_frame + 1).clamp(0, total - 1);
    player.current_frame = next_frame;
    player.current_time = next_frame as f64 / rate;
    if player.state == VideoPlayerState::Playing {
        player.state = VideoPlayerState::Paused;
    }
    if player.send_frame_ready_events {
        player.pending_events.frame_ready = true;
    }
    true
}

pub(crate) fn set_video_player_time_by_id(player_id: u64, seconds: f64) -> bool {
    if !seconds.is_finite() || seconds < 0.0 {
        return false;
    }
    let Some(player_arc) = get_player_arc(player_id) else {
        return false;
    };
    let Ok(mut player) = player_arc.lock() else {
        return false;
    };
    let _ = sync_worker_into_player(&mut player);
    player.current_time = seconds;
    if let Some(decoded) = player.decoded.as_ref() {
        let total = decoded.frames_rgba.len() as i64;
        let rate = decoded.frame_rate().max(1.0);
        if total > 0 {
            let idx = ((seconds * rate).floor() as i64).clamp(0, total - 1);
            player.current_frame = idx;
        }
    }
    player.pending_events.seek_completed = true;
    if player.send_frame_ready_events {
        player.pending_events.frame_ready = true;
    }
    true
}

pub(crate) fn set_video_player_frame_by_id(player_id: u64, frame_index: i64) -> bool {
    if frame_index < 0 {
        return false;
    }
    let Some(player_arc) = get_player_arc(player_id) else {
        return false;
    };
    let Ok(mut player) = player_arc.lock() else {
        return false;
    };
    let _ = sync_worker_into_player(&mut player);
    let frame_info = player.decoded.as_ref().map(|decoded| {
        (
            decoded.frames_rgba.len() as i64,
            decoded.frame_rate().max(1.0),
        )
    });
    if let Some((total, rate)) = frame_info {
        if total == 0 {
            return false;
        }
        let clamped = frame_index.clamp(0, total - 1);
        player.current_frame = clamped;
        player.current_time = clamped as f64 / rate;
    } else {
        player.current_frame = frame_index;
    }
    player.pending_events.seek_completed = true;
    if player.send_frame_ready_events {
        player.pending_events.frame_ready = true;
    }
    true
}

pub(crate) fn set_video_player_looping_by_id(player_id: u64, looping: bool) -> bool {
    let Some(player_arc) = get_player_arc(player_id) else {
        return false;
    };
    let Ok(mut player) = player_arc.lock() else {
        return false;
    };
    player.is_looping = looping;
    true
}

pub(crate) fn set_video_player_playback_speed_by_id(player_id: u64, speed: f32) -> bool {
    if !speed.is_finite() || speed < 0.0 {
        return false;
    }
    let Some(player_arc) = get_player_arc(player_id) else {
        return false;
    };
    let Ok(mut player) = player_arc.lock() else {
        return false;
    };
    player.playback_speed = speed;
    true
}

pub(crate) fn set_video_player_send_frame_ready_events_by_id(
    player_id: u64,
    enabled: bool,
) -> bool {
    let Some(player_arc) = get_player_arc(player_id) else {
        return false;
    };
    let Ok(mut player) = player_arc.lock() else {
        return false;
    };
    player.send_frame_ready_events = enabled;
    true
}

pub(crate) fn set_video_player_target_entity_by_id(player_id: u64, target: EntityId) -> bool {
    let Some(player_arc) = get_player_arc(player_id) else {
        return false;
    };
    let Ok(mut player) = player_arc.lock() else {
        return false;
    };
    player.target_entity = target;
    player.presented_frame = -1;
    true
}

pub(crate) fn advance_video_player_by_id(player_id: u64, delta_seconds: f64) -> bool {
    if !delta_seconds.is_finite() || delta_seconds < 0.0 {
        return false;
    }
    let Some(player_arc) = get_player_arc(player_id) else {
        return false;
    };
    let Ok(mut player) = player_arc.lock() else {
        return false;
    };
    let _ = sync_worker_into_player(&mut player);
    if player.state == VideoPlayerState::Playing
        && let Some(decoded) = player.decoded.clone()
    {
        advance_playing_state(&mut player, &decoded, delta_seconds);
    }
    true
}

pub(crate) fn take_video_player_snapshot_by_id(player_id: u64) -> Option<VideoPlayerSnapshot> {
    let player_arc = get_player_arc(player_id)?;
    let mut player = player_arc.lock().ok()?;
    let _ = sync_worker_into_player(&mut player);
    let snap = snapshot_from_player(&player);
    player.pending_events = VideoEvents::default();
    Some(snap)
}

pub(crate) fn video_player_current_frame_rgba_by_id(player_id: u64) -> Option<Vec<u8>> {
    let player_arc = get_player_arc(player_id)?;
    let mut player = player_arc.lock().ok()?;
    let _ = sync_worker_into_player(&mut player);
    let decoded = player.decoded.as_ref()?;
    let idx = usize::try_from(player.current_frame.max(0)).ok()?;
    decoded.frames_rgba.get(idx).cloned()
}

fn sync_worker_into_player(player: &mut VideoPlayerInternal) -> Option<String> {
    if player.decoded.is_some() {
        return None;
    }
    let taken = {
        let Ok(mut ws) = player.worker_state.lock() else {
            return None;
        };
        match &*ws {
            WorkerStatus::Ready(clip) => Some(Ok(Arc::clone(clip))),
            WorkerStatus::Failed(msg) => {
                let cloned = msg.clone();
                *ws = WorkerStatus::Idle;
                Some(Err(cloned))
            }
            _ => None,
        }
    }?;
    match taken {
        Ok(clip) => {
            player.decoded = Some(clip);
            player.pending_events.prepare_completed = true;
            if player.current_frame < 0 {
                player.current_frame = 0;
                player.current_time = 0.0;
            }
            if player.send_frame_ready_events {
                player.pending_events.frame_ready = true;
            }
            if player.play_when_prepared {
                player.play_when_prepared = false;
                player.state = VideoPlayerState::Playing;
                player.pending_events.started = true;
            } else {
                player.state = VideoPlayerState::Prepared;
            }
            None
        }
        Err(err_msg) => {
            player.state = VideoPlayerState::Failed;
            player.pending_events.error_received = true;
            player.last_error = Some(err_msg.clone());
            Some(err_msg)
        }
    }
}

fn advance_playing_state(
    player: &mut VideoPlayerInternal,
    decoded: &DecodedVideoClip,
    delta_seconds: f64,
) {
    let total_frames = decoded.frames_rgba.len();
    if total_frames == 0 {
        return;
    }
    let rate = decoded.frame_rate().max(1.0);
    let duration = (total_frames as f64) / rate;
    let scaled_delta = delta_seconds * f64::from(player.playback_speed);
    if scaled_delta <= 0.0 {
        return;
    }
    let prev_frame = player.current_frame;
    let mut new_time = player.current_time + scaled_delta;
    if duration > 0.0 && new_time >= duration {
        player.pending_events.loop_point_reached = true;
        if player.is_looping {
            new_time %= duration;
        } else {
            new_time = duration;
            player.state = VideoPlayerState::Paused;
        }
    }
    player.current_time = new_time;
    let computed_frame = ((new_time * rate).floor() as i64).clamp(0, (total_frames - 1) as i64);
    player.current_frame = computed_frame;
    if player.send_frame_ready_events && computed_frame != prev_frame {
        player.pending_events.frame_ready = true;
    }
}

fn snapshot_from_player(player: &VideoPlayerInternal) -> VideoPlayerSnapshot {
    let (width, height, frame_count, frame_rate, length_seconds) =
        if let Some(decoded) = player.decoded.as_ref() {
            (
                decoded.width,
                decoded.height,
                decoded.frame_count(),
                decoded.frame_rate(),
                decoded.duration_seconds(),
            )
        } else if let Some(cooked) = player.cooked.as_ref() {
            (
                cooked.width,
                cooked.height,
                cooked.frame_count,
                cooked.frame_rate(),
                cooked.duration_seconds(),
            )
        } else {
            (0, 0, 0, 0.0, 0.0)
        };
    VideoPlayerSnapshot {
        width,
        height,
        frame_count,
        frame_rate,
        length_seconds,
        current_frame: player.current_frame,
        current_time: player.current_time,
        playback_speed: player.playback_speed,
        is_prepared: player.decoded.is_some(),
        is_playing: player.state == VideoPlayerState::Playing,
        is_paused: player.state == VideoPlayerState::Paused,
        is_looping: player.is_looping,
        send_frame_ready_events: player.send_frame_ready_events,
        events: player.pending_events,
        last_error: player.last_error.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presented_video_frame_updates_sprite_texture_without_temp_file()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime = Runtime::new()?;
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("unitas_runtime_video_frames"));
        let entity = EntityId::new(7);
        let player_id = runtime.create_video_player(entity);
        let rgba: Vec<u8> = vec![10, 20, 30, 255, 40, 50, 60, 255];
        {
            let player_arc = get_player_arc(player_id).ok_or("player missing")?;
            let mut player = player_arc
                .lock()
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::Other))?;
            player.decoded = Some(Arc::new(DecodedVideoClip {
                width: 2,
                height: 1,
                fps_num: 30,
                fps_den: 1,
                duration_micros: 66_666,
                frames_rgba: vec![rgba.clone()],
            }));
        }
        assert!(runtime.play_video_player(player_id));
        let sprite = runtime
            .render
            .sprites
            .iter()
            .find(|sprite| sprite.entity == entity)
            .ok_or("sprite not created")?;
        let texture = sprite.texture.as_ref().ok_or("sprite has no texture")?;
        assert_eq!(texture.width, 2);
        assert_eq!(texture.height, 1);
        assert_eq!(texture.pixels, vec![[10, 20, 30], [40, 50, 60]]);
        assert_eq!(sprite.size, [0.02, 0.01]);
        assert!(
            !std::env::temp_dir()
                .join("unitas_runtime_video_frames")
                .exists()
        );
        Ok(())
    }
}
