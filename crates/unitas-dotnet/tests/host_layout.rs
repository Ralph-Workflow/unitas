//! Layout agreement across the .NET host zone.
//!
//! This zone calls hostfxr, so what crosses the boundary is a status code, C
//! strings, an out-parameter slot, an argument vector, and the C function
//! pointers hostfxr hands back. No struct crosses it: every one of those
//! types is a scalar or a pointer, and the crate's own `load_function`
//! refuses any delegate whose size or alignment does not match a function
//! pointer, which is the shape the managed `[UnmanagedCallersOnly]` entry
//! points must have to be resolved at all.

use std::ffi::{c_char, c_int, c_void};
use std::mem::{align_of, size_of};

type HostFxrInitialize =
    unsafe extern "C" fn(c_int, *const *const c_char, *const c_void, *mut *mut c_void) -> c_int;
type HostFxrGetFunctionPointer = unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    *const c_char,
    *mut c_void,
    *mut c_void,
    *mut *mut c_void,
) -> c_int;
type HostFxrGetDelegate = unsafe extern "C" fn(*mut c_void, c_int, *mut *mut c_void) -> c_int;

type PlayerHostInitEntry = extern "C" fn(u64, *const u8, *const u8, *const u8) -> i32;
type PlayerHostTickEntry = extern "C" fn(f64, *mut u8, u64, *mut u32, *mut u32, *mut u32) -> i32;
type PlayerHostShutdownEntry = extern "C" fn() -> i32;
type PlayerHostInjectInputEntry = extern "C" fn(u64, u64, u8, f64, f64) -> i32;

#[test]
fn hostfxr_status_codes_and_pointers_have_the_managed_widths() {
    assert_eq!(size_of::<c_int>(), 4);
    assert_eq!(align_of::<c_int>(), 4);
    assert_eq!(size_of::<*const c_char>(), size_of::<*mut c_void>());
    assert_eq!(align_of::<*const c_char>(), align_of::<*mut c_void>());
    assert_eq!(size_of::<*mut *mut c_void>(), size_of::<*mut c_void>());
    assert_eq!(align_of::<*mut *mut c_void>(), align_of::<*mut c_void>());
    assert_eq!(size_of::<*const *const c_char>(), size_of::<*mut c_void>());
    assert_eq!(size_of::<*mut c_void>(), size_of::<*mut u8>());
}

#[test]
fn hostfxr_entry_points_are_function_pointer_shaped() {
    assert_function_pointer::<HostFxrInitialize>();
    assert_function_pointer::<HostFxrGetFunctionPointer>();
    assert_function_pointer::<HostFxrGetDelegate>();
}

#[test]
fn managed_player_host_entries_are_function_pointer_shaped() {
    assert_function_pointer::<PlayerHostInitEntry>();
    assert_function_pointer::<PlayerHostTickEntry>();
    assert_function_pointer::<PlayerHostShutdownEntry>();
    assert_function_pointer::<PlayerHostInjectInputEntry>();
}

#[test]
fn managed_player_host_entries_use_the_managed_primitive_widths() {
    assert_eq!(size_of::<u64>(), 8);
    assert_eq!(align_of::<u64>(), 8);
    assert_eq!(size_of::<u8>(), 1);
    assert_eq!(align_of::<u8>(), 1);
    assert_eq!(size_of::<i32>(), 4);
    assert_eq!(align_of::<i32>(), 4);
    assert_eq!(size_of::<f64>(), 8);
    assert_eq!(align_of::<f64>(), 8);
    assert_eq!(size_of::<f32>(), 4);
    assert_eq!(align_of::<f32>(), 4);
    assert_eq!(size_of::<*const u8>(), size_of::<*mut u8>());
}

fn assert_function_pointer<T>() {
    assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
    assert_eq!(align_of::<T>(), align_of::<*mut c_void>());
}
