use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};

static SERIAL_CALLS: AtomicUsize = AtomicUsize::new(0);
static SERIAL_START: AtomicI64 = AtomicI64::new(-1);
static SERIAL_COUNT: AtomicI64 = AtomicI64::new(-1);
static PARALLEL_BATCHES: Mutex<Vec<(i64, i64)>> = Mutex::new(Vec::new());
static DEPENDENCY_FLAG: AtomicBool = AtomicBool::new(false);
static DEPENDENCY_VIOLATIONS: AtomicUsize = AtomicUsize::new(0);

extern "C" fn serial_callback(_ctx: *mut c_void, start: i64, count: i64) {
    SERIAL_CALLS.fetch_add(1, Ordering::SeqCst);
    SERIAL_START.store(start, Ordering::SeqCst);
    SERIAL_COUNT.store(count, Ordering::SeqCst);
}

extern "C" fn parallel_callback(_ctx: *mut c_void, start: i64, count: i64) {
    let mut batches = PARALLEL_BATCHES
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    batches.push((start, count));
}

extern "C" fn dependency_callback(_ctx: *mut c_void, _start: i64, _count: i64) {
    if !DEPENDENCY_FLAG.load(Ordering::SeqCst) {
        DEPENDENCY_VIOLATIONS.fetch_add(1, Ordering::SeqCst);
    }
}

extern "C" fn flag_callback(_ctx: *mut c_void, _start: i64, _count: i64) {
    DEPENDENCY_FLAG.store(true, Ordering::SeqCst);
}

extern "C" fn unused_callback(_ctx: *mut c_void, _start: i64, _count: i64) {}

#[test]
fn schedule_returns_a_valid_handle_that_completes() {
    let handle =
        unitas_runtime::unitas_runtime_jobs_schedule(serial_callback, std::ptr::null_mut(), 0);
    assert_ne!(handle, 0);
    unitas_runtime::unitas_runtime_jobs_complete(handle);
    assert_eq!(unitas_runtime::unitas_runtime_jobs_is_complete(handle), 1);
    assert_eq!(SERIAL_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(SERIAL_START.load(Ordering::SeqCst), 0);
    assert_eq!(SERIAL_COUNT.load(Ordering::SeqCst), 1);
}

#[test]
fn schedule_parallel_partitions_the_range() {
    let handle = unitas_runtime::unitas_runtime_jobs_schedule_parallel(
        parallel_callback,
        std::ptr::null_mut(),
        10,
        3,
        0,
    );
    assert_ne!(handle, 0);
    unitas_runtime::unitas_runtime_jobs_complete(handle);
    assert_eq!(unitas_runtime::unitas_runtime_jobs_is_complete(handle), 1);
    let batches = PARALLEL_BATCHES
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut covered = Vec::new();
    for (start, count) in batches.iter() {
        for index in *start..*start + *count {
            covered.push(index);
        }
    }
    covered.sort_unstable();
    assert_eq!(covered, (0..10).collect::<Vec<i64>>());
}

#[test]
fn a_dependency_gates_the_dependent_callback() {
    let dependency =
        unitas_runtime::unitas_runtime_jobs_schedule(flag_callback, std::ptr::null_mut(), 0);
    assert_ne!(dependency, 0);
    let dependent = unitas_runtime::unitas_runtime_jobs_schedule(
        dependency_callback,
        std::ptr::null_mut(),
        dependency,
    );
    assert_ne!(dependent, 0);
    unitas_runtime::unitas_runtime_jobs_complete(dependent);
    assert_eq!(DEPENDENCY_VIOLATIONS.load(Ordering::SeqCst), 0);
    assert!(DEPENDENCY_FLAG.load(Ordering::SeqCst));
    assert_eq!(
        unitas_runtime::unitas_runtime_jobs_is_complete(dependency),
        1
    );
}

#[test]
fn handle_zero_is_invalid() {
    assert_eq!(unitas_runtime::unitas_runtime_jobs_is_complete(0), 0);
    unitas_runtime::unitas_runtime_jobs_complete(0);
    let handle =
        unitas_runtime::unitas_runtime_jobs_schedule(unused_callback, std::ptr::null_mut(), 9_999);
    assert_eq!(handle, 0);
}
