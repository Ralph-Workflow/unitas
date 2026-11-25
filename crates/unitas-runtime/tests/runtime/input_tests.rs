use std::error::Error;
use std::path::Path;

use unitas_runtime::{
    default_axis_configs, parse_button_token, test_read_path_buffer, test_write_path_buffer,
    unitas_runtime_destroy, unitas_runtime_error_message, unitas_runtime_input_axis,
    unitas_runtime_input_axis_raw, unitas_runtime_input_button_state,
    unitas_runtime_input_key_event, unitas_runtime_input_key_state,
    unitas_runtime_input_mouse_button_event, unitas_runtime_input_mouse_state,
    unitas_runtime_input_pointer, unitas_runtime_input_pointer_move, unitas_runtime_input_scroll,
    unitas_runtime_input_scroll_state, unitas_runtime_load_scene, unitas_runtime_new,
    unitas_runtime_path_buffer, unitas_runtime_tick, AxisConfig, ButtonSource, InputState, Runtime,
    STATE_DOWN, STATE_PRESSED, STATE_RELEASED,
};

#[test]
fn input_parse_button_token_maps_unity_names_and_mouse_buttons_case_insensitively() {
    assert_eq!(parse_button_token("a"), Some(ButtonSource::Key(97)));
    assert_eq!(parse_button_token("Z"), Some(ButtonSource::Key(122)));
    assert_eq!(parse_button_token("0"), Some(ButtonSource::Key(48)));
    assert_eq!(parse_button_token("9"), Some(ButtonSource::Key(57)));
    assert_eq!(parse_button_token("space"), Some(ButtonSource::Key(32)));
    assert_eq!(parse_button_token("  Escape "), Some(ButtonSource::Key(27)));
    assert_eq!(parse_button_token("return"), Some(ButtonSource::Key(13)));
    assert_eq!(parse_button_token("tab"), Some(ButtonSource::Key(9)));
    assert_eq!(parse_button_token("backspace"), Some(ButtonSource::Key(8)));
    assert_eq!(parse_button_token("delete"), Some(ButtonSource::Key(127)));
    assert_eq!(parse_button_token("up"), Some(ButtonSource::Key(273)));
    assert_eq!(parse_button_token("down"), Some(ButtonSource::Key(274)));
    assert_eq!(parse_button_token("right"), Some(ButtonSource::Key(275)));
    assert_eq!(parse_button_token("left"), Some(ButtonSource::Key(276)));
    assert_eq!(
        parse_button_token("right shift"),
        Some(ButtonSource::Key(303))
    );
    assert_eq!(
        parse_button_token("left shift"),
        Some(ButtonSource::Key(304))
    );
    assert_eq!(
        parse_button_token("right ctrl"),
        Some(ButtonSource::Key(305))
    );
    assert_eq!(
        parse_button_token("left ctrl"),
        Some(ButtonSource::Key(306))
    );
    assert_eq!(
        parse_button_token("right alt"),
        Some(ButtonSource::Key(307))
    );
    assert_eq!(parse_button_token("left alt"), Some(ButtonSource::Key(308)));
    assert_eq!(
        parse_button_token("mouse 0"),
        Some(ButtonSource::MouseButton(0))
    );
    assert_eq!(
        parse_button_token("Mouse 1"),
        Some(ButtonSource::MouseButton(1))
    );
    assert_eq!(
        parse_button_token("mouse 2"),
        Some(ButtonSource::MouseButton(2))
    );
    assert_eq!(parse_button_token(""), None);
    assert_eq!(parse_button_token("unknown_key"), None);
}

#[test]
fn input_key_and_mouse_transitions_hold_for_one_frame_and_ignore_redundant_events() {
    let mut input = InputState::new();

    input.queue_key_event(32, true);
    input.queue_mouse_button_event(0, true);
    assert_eq!(input.key_state(32), 0);
    assert_eq!(input.mouse_button_state(0), 0);

    input.begin_frame(0.02);
    assert_eq!(input.key_state(32), STATE_DOWN | STATE_PRESSED);
    assert_eq!(input.mouse_button_state(0), STATE_DOWN | STATE_PRESSED);
    assert_eq!(input.key_state(323), STATE_DOWN | STATE_PRESSED);

    input.queue_key_event(32, true);
    input.begin_frame(0.02);
    assert_eq!(input.key_state(32), STATE_DOWN);
    assert_eq!(input.mouse_button_state(0), STATE_DOWN);

    input.queue_key_event(32, false);
    input.queue_mouse_button_event(0, false);
    input.begin_frame(0.02);
    assert_eq!(input.key_state(32), STATE_RELEASED);
    assert_eq!(input.mouse_button_state(0), STATE_RELEASED);
    assert_eq!(input.key_state(323), STATE_RELEASED);

    input.queue_key_event(32, false);
    input.begin_frame(0.02);
    assert_eq!(input.key_state(32), 0);
    assert_eq!(input.mouse_button_state(0), 0);

    input.queue_key_event(97, true);
    input.queue_key_event(97, false);
    input.begin_frame(0.02);
    assert_eq!(input.key_state(97), STATE_PRESSED | STATE_RELEASED);

    input.begin_frame(0.02);
    assert_eq!(input.key_state(97), 0);
}

#[test]
fn input_pointer_move_and_scroll_accumulate_per_frame_and_reset_deltas() {
    let mut input = InputState::new();
    assert_eq!(input.pointer(), (0.0, 0.0, 0.0, 0.0));
    assert_eq!(input.scroll(), (0.0, 0.0));

    input.queue_pointer_move(10.0, 20.0);
    input.queue_pointer_move(15.0, 35.0);
    input.queue_scroll(1.0, -2.0);
    input.queue_scroll(0.5, -1.5);
    input.begin_frame(0.016);

    assert_eq!(input.pointer(), (15.0, 35.0, 15.0, 35.0));
    assert_eq!(input.scroll(), (1.5, -3.5));

    input.begin_frame(0.016);
    assert_eq!(input.pointer(), (15.0, 35.0, 0.0, 0.0));
    assert_eq!(input.scroll(), (0.0, 0.0));

    input.queue_pointer_move(12.0, 40.0);
    input.begin_frame(0.016);
    assert_eq!(input.pointer(), (12.0, 40.0, -3.0, 5.0));
}

#[test]
fn input_key_axis_ramps_with_sensitivity_decays_with_gravity_and_respects_snap_invert_and_deadzone()
{
    let mut input = InputState::new();
    input.set_axes(vec![
        AxisConfig {
            name: String::from("Horizontal"),
            negative_button: String::from("left"),
            positive_button: String::from("right"),
            alt_negative_button: String::from("a"),
            alt_positive_button: String::from("d"),
            gravity: 3.0,
            dead: 0.05,
            sensitivity: 3.0,
            snap: true,
            invert: false,
            axis_type: 0,
            axis: 0,
        },
        AxisConfig {
            name: String::from("NoSnapInverted"),
            negative_button: String::from("down"),
            positive_button: String::from("up"),
            alt_negative_button: String::new(),
            alt_positive_button: String::new(),
            gravity: 2.0,
            dead: 0.25,
            sensitivity: 2.0,
            snap: false,
            invert: true,
            axis_type: 0,
            axis: 0,
        },
    ]);

    assert_eq!(input.axis("UnknownAxis"), None);
    assert_eq!(input.axis_raw("UnknownAxis"), None);
    assert_eq!(input.button_state("UnknownAxis"), None);
    assert_eq!(input.axis("Horizontal"), Some(0.0));

    input.queue_key_event(275, true);
    input.begin_frame(0.1);
    let h1 = input.axis("Horizontal").unwrap_or(-999.0);
    assert!((h1 - 0.3).abs() < 1e-9);
    assert_eq!(input.axis_raw("Horizontal"), Some(1.0));
    assert_eq!(
        input.button_state("Horizontal"),
        Some(STATE_DOWN | STATE_PRESSED)
    );

    input.begin_frame(0.1);
    let h2 = input.axis("Horizontal").unwrap_or(-999.0);
    assert!((h2 - 0.6).abs() < 1e-9);
    assert_eq!(input.button_state("Horizontal"), Some(STATE_DOWN));

    input.begin_frame(0.2);
    let h3 = input.axis("Horizontal").unwrap_or(-999.0);
    assert!((h3 - 1.0).abs() < 1e-9);

    input.queue_key_event(275, false);
    input.queue_key_event(97, true);
    input.begin_frame(0.1);
    let h_reversed = input.axis("Horizontal").unwrap_or(-999.0);
    assert!((h_reversed - (-0.3)).abs() < 1e-9);
    assert_eq!(input.axis_raw("Horizontal"), Some(-1.0));

    input.queue_key_event(97, false);
    input.begin_frame(0.09);
    let h_dead = input.axis("Horizontal").unwrap_or(-999.0);
    assert_eq!(h_dead, 0.0);
    assert_eq!(input.axis_raw("Horizontal"), Some(0.0));
    assert_eq!(input.button_state("Horizontal"), Some(STATE_RELEASED));

    input.queue_key_event(273, true);
    input.begin_frame(0.1);
    let inv_dead = input.axis("NoSnapInverted").unwrap_or(-999.0);
    assert_eq!(inv_dead, 0.0);
    assert_eq!(input.axis_raw("NoSnapInverted"), Some(-1.0));

    input.begin_frame(0.2);
    let inv_active = input.axis("NoSnapInverted").unwrap_or(-999.0);
    assert!((inv_active - (-0.6)).abs() < 1e-9);

    input.queue_key_event(273, false);
    input.queue_key_event(274, true);
    input.begin_frame(0.1);
    let inv_no_snap = input.axis("NoSnapInverted").unwrap_or(-999.0);
    assert!((inv_no_snap - (-0.4)).abs() < 1e-9);
}

#[test]
fn input_default_axis_configs_match_unity_project_settings_defaults() {
    let axes = default_axis_configs();
    let horizontal = axes.iter().find(|axis| axis.name == "Horizontal");
    let vertical = axes.iter().find(|axis| axis.name == "Vertical");
    assert!(
        horizontal.is_some(),
        "the fallback axis set must contain Horizontal"
    );
    assert!(
        vertical.is_some(),
        "the fallback axis set must contain Vertical"
    );
    assert_eq!(
        horizontal.map(|axis| axis.snap),
        Some(true),
        "Unity's default Horizontal axis snaps"
    );
    assert_eq!(
        vertical.map(|axis| axis.snap),
        Some(false),
        "Unity's default Vertical axis does not snap"
    );
}

#[test]
fn input_mouse_delta_axes_multiple_same_name_entries_and_reset_clear_all_state() {
    let mut input = InputState::with_default_axes();
    assert!(!default_axis_configs().is_empty());

    input.set_axes(vec![
        AxisConfig {
            name: String::from("Mouse X"),
            negative_button: String::new(),
            positive_button: String::new(),
            alt_negative_button: String::new(),
            alt_positive_button: String::new(),
            gravity: 0.0,
            dead: 0.0,
            sensitivity: 0.1,
            snap: false,
            invert: false,
            axis_type: 1,
            axis: 0,
        },
        AxisConfig {
            name: String::from("Mouse ScrollWheel"),
            negative_button: String::new(),
            positive_button: String::new(),
            alt_negative_button: String::new(),
            alt_positive_button: String::new(),
            gravity: 0.0,
            dead: 0.0,
            sensitivity: 0.5,
            snap: false,
            invert: true,
            axis_type: 1,
            axis: 2,
        },
        AxisConfig {
            name: String::from("DualAxis"),
            negative_button: String::from("left"),
            positive_button: String::from("right"),
            alt_negative_button: String::new(),
            alt_positive_button: String::new(),
            gravity: 3.0,
            dead: 0.0,
            sensitivity: 2.0,
            snap: true,
            invert: false,
            axis_type: 0,
            axis: 0,
        },
        AxisConfig {
            name: String::from("DualAxis"),
            negative_button: String::from("a"),
            positive_button: String::from("d"),
            alt_negative_button: String::new(),
            alt_positive_button: String::new(),
            gravity: 3.0,
            dead: 0.0,
            sensitivity: 5.0,
            snap: true,
            invert: false,
            axis_type: 0,
            axis: 0,
        },
    ]);

    input.queue_pointer_move(20.0, 5.0);
    input.queue_scroll(0.0, 4.0);
    input.queue_key_event(32, true);
    input.queue_key_event(100, true);
    input.begin_frame(0.1);

    let mx = input.axis("Mouse X").unwrap_or(-999.0);
    assert!((mx - 2.0).abs() < 1e-9);
    assert_eq!(input.axis_raw("Mouse X"), Some(20.0));

    let wheel = input.axis("Mouse ScrollWheel").unwrap_or(-999.0);
    assert!((wheel - (-2.0)).abs() < 1e-9);
    assert_eq!(input.axis_raw("Mouse ScrollWheel"), Some(-4.0));

    let dual = input.axis("DualAxis").unwrap_or(-999.0);
    assert!((dual - 0.5).abs() < 1e-9);

    input.reset();
    assert_eq!(input.key_state(32), 0);
    assert_eq!(input.pointer(), (0.0, 0.0, 0.0, 0.0));
    assert_eq!(input.scroll(), (0.0, 0.0));
    assert_eq!(input.axis("Mouse X"), Some(0.0));
    assert_eq!(input.axis("DualAxis"), Some(0.0));
}

#[test]
fn runtime_discovers_input_manager_asset_on_load_scene_and_resets_input_on_reload(
) -> Result<(), Box<dyn Error>> {
    let temp_root = std::env::temp_dir().join(format!(
        "unitas_runtime_input_manager_discovery_{}",
        std::process::id()
    ));
    let settings_dir = temp_root.join("ProjectSettings");
    let scenes_dir = temp_root.join("Assets").join("Scenes");
    std::fs::create_dir_all(&settings_dir)?;
    std::fs::create_dir_all(&scenes_dir)?;
    std::fs::write(
        settings_dir.join("InputManager.asset"),
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!13 &1\nInputManager:\n  m_Axes:\n  - serializedVersion: 3\n    m_Name: CustomThrust\n    negativeButton: s\n    positiveButton: w\n    altNegativeButton: \n    altPositiveButton: \n    gravity: 4\n    dead: 0.01\n    sensitivity: 4\n    snap: 1\n    invert: 0\n    type: 0\n    axis: 0\n",
    )?;
    let temp_scene = scenes_dir.join("Scene.unity");
    std::fs::copy(
        Path::new("tests/fixtures/scene_with_script.unity"),
        &temp_scene,
    )?;

    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&temp_scene);
    if let Err(err) = load_res {
        let _ = std::fs::remove_dir_all(&temp_root);
        return Err(err.into());
    }

    assert_eq!(runtime.input_axis("CustomThrust"), Some(0.0));
    assert_eq!(runtime.input_axis("Horizontal"), None);

    runtime.input_key_event(119, true);
    runtime.tick(0.0)?;
    runtime.tick(0.1)?;

    assert_eq!(runtime.input_key_state(119), STATE_DOWN | STATE_PRESSED);
    let thrust = runtime.input_axis("CustomThrust").unwrap_or(-999.0);
    assert!((thrust - 0.4).abs() < 1e-9);
    assert_eq!(runtime.input_axis_raw("CustomThrust"), Some(1.0));
    assert_eq!(
        runtime.input_button_state("CustomThrust"),
        Some(STATE_DOWN | STATE_PRESSED)
    );

    runtime.input_key_event(32, true);
    runtime.load_scene(Path::new("tests/fixtures/scene_with_script.unity"))?;
    let _ = std::fs::remove_dir_all(&temp_root);

    assert_eq!(runtime.input_key_state(119), 0);
    assert_eq!(runtime.input_axis("CustomThrust"), None);
    assert_eq!(runtime.input_axis("Horizontal"), Some(0.0));
    assert_eq!(runtime.input_key_state(32), 0);
    runtime.tick(0.1)?;
    runtime.tick(0.1)?;
    assert_eq!(
        runtime.input_key_state(32),
        0,
        "a key press queued before a scene load must never reach a frame boundary"
    );
    Ok(())
}

#[test]
fn runtime_abi_input_injection_and_queries_round_trip_across_frames() -> Result<(), Box<dyn Error>>
{
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let buf_addr = unitas_runtime_path_buffer(handle, 256);
    assert_ne!(buf_addr, 0);

    let scene_bytes = b"tests/fixtures/scene_with_script.unity";
    test_write_path_buffer(handle, scene_bytes)?;
    assert_eq!(
        unitas_runtime_load_scene(handle, buf_addr, scene_bytes.len() as u64),
        0
    );

    assert_eq!(unitas_runtime_tick(handle, 0.0), 0);

    assert_eq!(unitas_runtime_input_key_event(handle, 32, 1), 0);
    assert_eq!(unitas_runtime_input_key_event(handle, 100, 1), 0);
    assert_eq!(unitas_runtime_input_mouse_button_event(handle, 0, 1), 0);
    assert_eq!(unitas_runtime_input_pointer_move(handle, 120.5, 80.25), 0);
    assert_eq!(unitas_runtime_input_scroll(handle, 1.5, -2.5), 0);

    assert_eq!(unitas_runtime_input_key_state(handle, 32), 0);
    assert_eq!(unitas_runtime_input_mouse_state(handle, 0), 0);

    assert_eq!(unitas_runtime_tick(handle, 0.1), 0);

    assert_eq!(
        unitas_runtime_input_key_state(handle, 32),
        STATE_DOWN | STATE_PRESSED
    );
    assert_eq!(
        unitas_runtime_input_mouse_state(handle, 0),
        STATE_DOWN | STATE_PRESSED
    );

    assert_eq!(unitas_runtime_input_pointer(handle, buf_addr), 0);
    let mut ptr_bytes = [0u8; 32];
    test_read_path_buffer(handle, &mut ptr_bytes)?;
    let px = f64::from_le_bytes(ptr_bytes[0..8].try_into()?);
    let py = f64::from_le_bytes(ptr_bytes[8..16].try_into()?);
    let pdx = f64::from_le_bytes(ptr_bytes[16..24].try_into()?);
    let pdy = f64::from_le_bytes(ptr_bytes[24..32].try_into()?);
    assert_eq!((px, py, pdx, pdy), (120.5, 80.25, 120.5, 80.25));

    assert_eq!(unitas_runtime_input_scroll_state(handle, buf_addr), 0);
    let mut scroll_bytes = [0u8; 16];
    test_read_path_buffer(handle, &mut scroll_bytes)?;
    let sx = f64::from_le_bytes(scroll_bytes[0..8].try_into()?);
    let sy = f64::from_le_bytes(scroll_bytes[8..16].try_into()?);
    assert_eq!((sx, sy), (1.5, -2.5));

    let horiz_name = b"Horizontal";
    test_write_path_buffer(handle, horiz_name)?;
    let horiz = unitas_runtime_input_axis(handle, buf_addr, horiz_name.len() as u64);
    assert!((horiz - 0.3).abs() < 1e-9);
    assert_eq!(unitas_runtime_error_message(handle), 0);
    let horiz_raw = unitas_runtime_input_axis_raw(handle, buf_addr, horiz_name.len() as u64);
    assert_eq!(horiz_raw, 1.0);
    assert_eq!(unitas_runtime_error_message(handle), 0);

    let jump_name = b"Jump";
    test_write_path_buffer(handle, jump_name)?;
    assert_eq!(
        unitas_runtime_input_button_state(handle, buf_addr, jump_name.len() as u64),
        (STATE_DOWN | STATE_PRESSED) as i32
    );
    assert_eq!(unitas_runtime_error_message(handle), 0);

    let unknown_name = b"NoSuchAxis";
    test_write_path_buffer(handle, unknown_name)?;
    assert_eq!(
        unitas_runtime_input_axis(handle, buf_addr, unknown_name.len() as u64),
        0.0
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(
        unitas_runtime_input_axis_raw(handle, buf_addr, unknown_name.len() as u64),
        0.0
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(
        unitas_runtime_input_button_state(handle, buf_addr, unknown_name.len() as u64),
        -4
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);

    assert_eq!(unitas_runtime_tick(handle, 0.1), 0);
    assert_eq!(unitas_runtime_input_key_state(handle, 32), STATE_DOWN);
    assert_eq!(unitas_runtime_input_mouse_state(handle, 0), STATE_DOWN);

    assert_eq!(unitas_runtime_input_pointer(handle, buf_addr), 0);
    test_read_path_buffer(handle, &mut ptr_bytes)?;
    let pdx2 = f64::from_le_bytes(ptr_bytes[16..24].try_into()?);
    let pdy2 = f64::from_le_bytes(ptr_bytes[24..32].try_into()?);
    assert_eq!((pdx2, pdy2), (0.0, 0.0));

    assert_eq!(unitas_runtime_input_scroll_state(handle, buf_addr), 0);
    test_read_path_buffer(handle, &mut scroll_bytes)?;
    let sx2 = f64::from_le_bytes(scroll_bytes[0..8].try_into()?);
    let sy2 = f64::from_le_bytes(scroll_bytes[8..16].try_into()?);
    assert_eq!((sx2, sy2), (0.0, 0.0));

    assert_eq!(unitas_runtime_input_key_event(u64::MAX, 32, 1), -1);
    assert_eq!(unitas_runtime_input_mouse_button_event(u64::MAX, 0, 1), -1);
    assert_eq!(unitas_runtime_input_pointer_move(u64::MAX, 0.0, 0.0), -1);
    assert_eq!(unitas_runtime_input_scroll(u64::MAX, 0.0, 0.0), -1);
    assert_eq!(unitas_runtime_input_pointer_move(handle, f64::NAN, 0.0), -2);
    assert_eq!(unitas_runtime_input_scroll(handle, 0.0, f64::INFINITY), -2);
    assert_eq!(unitas_runtime_input_pointer(handle, 0), -2);
    assert_eq!(unitas_runtime_input_scroll_state(handle, 0), -2);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn runtime_abi_supports_reentrant_input_queries_inside_tick_callbacks() -> Result<(), Box<dyn Error>>
{
    use std::sync::atomic::{AtomicU64, Ordering};
    use unitas_runtime::unitas_runtime_set_callbacks;

    static REENTRANT_HANDLE: AtomicU64 = AtomicU64::new(0);
    static OBSERVED_KEY_MASK: AtomicU64 = AtomicU64::new(0);
    static OBSERVED_AXIS_BITS: AtomicU64 = AtomicU64::new(0);

    extern "C" fn on_create(_entity: u64, _hi: u64, _lo: u64) -> u64 {
        1
    }
    extern "C" fn noop_u64(_script: u64) {}
    extern "C" fn noop_fixed(_script: u64, _dt: f64) {}
    extern "C" fn on_update(_script: u64, _dt: f64) {
        let h = REENTRANT_HANDLE.load(Ordering::SeqCst);
        let mask = unitas_runtime_input_key_state(h, 32);
        OBSERVED_KEY_MASK.store(u64::from(mask), Ordering::SeqCst);
        let name = b"Horizontal";
        let buf = unitas_runtime_path_buffer(h, name.len() as u64);
        if buf != 0 && test_write_path_buffer(h, name).is_ok() {
            let val = unitas_runtime_input_axis(h, buf, name.len() as u64);
            OBSERVED_AXIS_BITS.store(val.to_bits(), Ordering::SeqCst);
        }
    }
    extern "C" fn noop_collision(_a: u64, _b: u64, _phase: u32) {}

    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    REENTRANT_HANDLE.store(handle, Ordering::SeqCst);

    assert_eq!(
        unitas_runtime_set_callbacks(
            handle,
            on_create,
            noop_u64,
            noop_u64,
            noop_u64,
            noop_fixed,
            on_update,
            noop_fixed,
            noop_u64,
            noop_u64,
            noop_collision,
            noop_collision,
        ),
        0
    );

    let scene_bytes = b"tests/fixtures/scene_with_script.unity";
    let buf_addr = unitas_runtime_path_buffer(handle, scene_bytes.len() as u64);
    test_write_path_buffer(handle, scene_bytes)?;
    assert_eq!(
        unitas_runtime_load_scene(handle, buf_addr, scene_bytes.len() as u64),
        0
    );

    assert_eq!(unitas_runtime_tick(handle, 0.0), 0);
    assert_eq!(unitas_runtime_input_key_event(handle, 32, 1), 0);
    assert_eq!(unitas_runtime_input_key_event(handle, 275, 1), 0);
    assert_eq!(unitas_runtime_tick(handle, 0.1), 0);

    assert_eq!(
        OBSERVED_KEY_MASK.load(Ordering::SeqCst),
        u64::from(STATE_DOWN | STATE_PRESSED)
    );
    let observed_axis = f64::from_bits(OBSERVED_AXIS_BITS.load(Ordering::SeqCst));
    assert!((observed_axis - 0.3).abs() < 1e-9);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}
