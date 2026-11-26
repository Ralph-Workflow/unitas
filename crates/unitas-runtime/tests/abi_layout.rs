//! Layout agreement across the runtime ABI zone.
//!
//! Two `#[repr(C)]` structs cross this boundary by pointer, the exported
//! signatures cross as fixed-width scalars, the script callbacks cross as C
//! function pointers, and the query, camera, and animator payloads cross as
//! fixed-size byte records. Each of those layouts is pinned here and on the
//! managed side in `editor/Unitas.Editor.Tests/AbiLayoutTests.cs`. The trail
//! settings blob crosses this boundary too, but no managed caller reads it,
//! so it is pinned here alone.

use std::mem::{align_of, offset_of, size_of};

use unitas_runtime::{
    AbiCharacterMoveInput, AbiCharacterMoveOutput, Callbacks, RuntimeError, TRAIL_SETTINGS_FLOATS,
    test_read_query_buffer, unitas_runtime_destroy, unitas_runtime_new,
};

const QUERY_RECORD_BYTES: usize = 64;
const CAMERA_BLOB_BYTES: usize = 32;
const ANIMATOR_STATE_INFO_BYTES: usize = 80;

type CreateScriptCallback = extern "C" fn(u64, u64, u64) -> u64;
type ScriptCallback = extern "C" fn(u64);
type TimedCallback = extern "C" fn(u64, f64);
type PhaseCallback = extern "C" fn(u64, u64, u32);

const _: fn(&Callbacks) = assert_callback_signatures;

fn assert_callback_signatures(callbacks: &Callbacks) {
    let _: CreateScriptCallback = callbacks.create_script;
    let _: ScriptCallback = callbacks.awake;
    let _: ScriptCallback = callbacks.on_enable;
    let _: ScriptCallback = callbacks.start;
    let _: TimedCallback = callbacks.fixed_update;
    let _: TimedCallback = callbacks.update;
    let _: TimedCallback = callbacks.late_update;
    let _: ScriptCallback = callbacks.on_disable;
    let _: ScriptCallback = callbacks.destroy;
    let _: PhaseCallback = callbacks.collision;
    let _: PhaseCallback = callbacks.trigger;
}

#[test]
fn character_move_input_layout_matches_the_managed_struct() {
    assert_eq!(size_of::<AbiCharacterMoveInput>(), 144);
    assert_eq!(align_of::<AbiCharacterMoveInput>(), 8);
    assert_eq!(offset_of!(AbiCharacterMoveInput, entity_id), 0);
    assert_eq!(offset_of!(AbiCharacterMoveInput, position), 8);
    assert_eq!(offset_of!(AbiCharacterMoveInput, motion), 32);
    assert_eq!(offset_of!(AbiCharacterMoveInput, radius), 56);
    assert_eq!(offset_of!(AbiCharacterMoveInput, height), 64);
    assert_eq!(offset_of!(AbiCharacterMoveInput, center), 72);
    assert_eq!(offset_of!(AbiCharacterMoveInput, slope_limit_deg), 96);
    assert_eq!(offset_of!(AbiCharacterMoveInput, step_offset), 104);
    assert_eq!(offset_of!(AbiCharacterMoveInput, skin_width), 112);
    assert_eq!(offset_of!(AbiCharacterMoveInput, min_move_distance), 120);
    assert_eq!(offset_of!(AbiCharacterMoveInput, delta_seconds), 128);
    assert_eq!(offset_of!(AbiCharacterMoveInput, layer_mask), 136);
    assert_eq!(offset_of!(AbiCharacterMoveInput, enabled), 140);
    assert_eq!(offset_of!(AbiCharacterMoveInput, reserved), 141);
}

#[test]
fn character_move_output_layout_matches_the_managed_struct() {
    assert_eq!(size_of::<AbiCharacterMoveOutput>(), 40);
    assert_eq!(align_of::<AbiCharacterMoveOutput>(), 8);
    assert_eq!(offset_of!(AbiCharacterMoveOutput, position), 0);
    assert_eq!(offset_of!(AbiCharacterMoveOutput, ground_entity_id), 24);
    assert_eq!(offset_of!(AbiCharacterMoveOutput, collision_flags), 32);
    assert_eq!(offset_of!(AbiCharacterMoveOutput, is_grounded), 36);
    assert_eq!(offset_of!(AbiCharacterMoveOutput, reserved), 37);
}

#[test]
fn exported_scalar_widths_match_the_managed_primitives() {
    assert_eq!(size_of::<u8>(), 1);
    assert_eq!(align_of::<u8>(), 1);
    assert_eq!(size_of::<u32>(), 4);
    assert_eq!(align_of::<u32>(), 4);
    assert_eq!(size_of::<u64>(), 8);
    assert_eq!(align_of::<u64>(), 8);
    assert_eq!(size_of::<i32>(), 4);
    assert_eq!(align_of::<i32>(), 4);
    assert_eq!(size_of::<f32>(), 4);
    assert_eq!(align_of::<f32>(), 4);
    assert_eq!(size_of::<f64>(), 8);
    assert_eq!(align_of::<f64>(), 8);
}

#[test]
fn script_callbacks_are_function_pointer_shaped() {
    assert_function_pointer::<CreateScriptCallback>();
    assert_function_pointer::<ScriptCallback>();
    assert_function_pointer::<TimedCallback>();
    assert_function_pointer::<PhaseCallback>();
    assert_eq!(size_of::<Callbacks>(), 11 * size_of::<*mut u8>());
}

fn assert_function_pointer<T>() {
    assert_eq!(size_of::<T>(), size_of::<*mut u8>());
    assert_eq!(align_of::<T>(), align_of::<*mut u8>());
}

#[test]
fn blob_record_sizes_match_the_managed_buffers() {
    assert_eq!(QUERY_RECORD_BYTES, size_of::<[u64; 8]>());
    assert_eq!(CAMERA_BLOB_BYTES, size_of::<[f32; 8]>());
    assert_eq!(ANIMATOR_STATE_INFO_BYTES, size_of::<[u32; 20]>());
    assert_eq!(
        TRAIL_SETTINGS_FLOATS * size_of::<f32>(),
        size_of::<[f32; 12]>()
    );
}

#[test]
fn query_record_stride_is_sixty_four_bytes() {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0, "runtime handle creation failed");
    for partial in [32usize, 63] {
        let mut bytes = vec![0u8; partial];
        assert!(
            matches!(
                test_read_query_buffer(handle, &mut bytes),
                Err(RuntimeError::InvalidAddress)
            ),
            "a query buffer of {partial} bytes must be refused as a partial record"
        );
    }
    for whole in [64usize, 128] {
        let mut bytes = vec![0u8; whole];
        assert!(
            test_read_query_buffer(handle, &mut bytes).is_ok(),
            "a query buffer of {whole} bytes must be accepted as whole records"
        );
    }
    assert_eq!(unitas_runtime_destroy(handle), 0);
}
