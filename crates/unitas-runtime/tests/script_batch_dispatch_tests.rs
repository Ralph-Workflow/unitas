use std::cell::RefCell;
use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

use unitas_runtime::{Callbacks, Runtime};

const EMPTY_SCENE_FIXTURE: &str = "tests/fixtures/scene_physics_collision_no_script.unity";

const PHASE_FIXED_UPDATE: u32 = 0;
const PHASE_UPDATE: u32 = 1;
const PHASE_LATE_UPDATE: u32 = 2;

thread_local! {
    static NEXT_HANDLE_OFFSET: RefCell<u64> = const { RefCell::new(1000) };
    static BATCH_CALLS: RefCell<Vec<BatchCall>> = const { RefCell::new(Vec::new()) };
    static PER_SCRIPT_UPDATE_CALLS: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static FAULTING: RefCell<BTreeSet<u64>> = const { RefCell::new(BTreeSet::new()) };
    static DISPATCHED: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static FAULTED: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

fn reset_state() {
    NEXT_HANDLE_OFFSET.with(|cell| *cell.borrow_mut() = 1000);
    BATCH_CALLS.with(|cell| cell.borrow_mut().clear());
    PER_SCRIPT_UPDATE_CALLS.with(|cell| cell.borrow_mut().clear());
    FAULTING.with(|cell| cell.borrow_mut().clear());
    DISPATCHED.with(|cell| cell.borrow_mut().clear());
    FAULTED.with(|cell| cell.borrow_mut().clear());
}

fn set_faulting(handle: u64) {
    FAULTING.with(|cell| cell.borrow_mut().insert(handle));
}

fn run_body(handle: u64) -> Result<(), ()> {
    if FAULTING.with(|cell| cell.borrow().contains(&handle)) {
        return Err(());
    }
    DISPATCHED.with(|cell| cell.borrow_mut().push(handle));
    Ok(())
}

type BatchCall = (u32, Vec<u64>, Vec<f64>);

fn take_batch_calls() -> Vec<BatchCall> {
    BATCH_CALLS.with(|cell| cell.borrow().clone())
}

fn batch_calls_for(kind: u32) -> Vec<BatchCall> {
    take_batch_calls()
        .into_iter()
        .filter(|(call_kind, _, _)| *call_kind == kind)
        .collect()
}

fn dispatched() -> Vec<u64> {
    DISPATCHED.with(|cell| cell.borrow().clone())
}

fn faulted() -> Vec<u64> {
    FAULTED.with(|cell| cell.borrow().clone())
}

extern "C" fn counting_create_script(entity: u64, _guid_hi: u64, _guid_lo: u64) -> u64 {
    let offset = NEXT_HANDLE_OFFSET.with(|cell| {
        let mut current = cell.borrow_mut();
        let value = *current;
        *current += 1000;
        value
    });
    entity.saturating_add(offset)
}

extern "C" fn noop_handle(_handle: u64) {}

extern "C" fn noop_handle_delta(_handle: u64, _delta: f64) {}

extern "C" fn noop_collision(_a: u64, _b: u64, _phase: u32) {}

extern "C" fn counting_update(handle: u64, _delta: f64) {
    PER_SCRIPT_UPDATE_CALLS.with(|cell| cell.borrow_mut().push(handle));
}

extern "C" fn isolating_update(handle: u64, _delta: f64) {
    if run_body(handle).is_err() {
        FAULTED.with(|cell| cell.borrow_mut().push(handle));
    }
}

extern "C" fn counting_batch(kind: u32, handles: *const u64, deltas: *const f64, len: u64) {
    let len = usize::try_from(len).unwrap_or(0);
    // SAFETY: the runtime passes arrays of exactly `len` valid elements for the batch.
    let handles = unsafe { std::slice::from_raw_parts(handles, len) };
    // SAFETY: the runtime passes arrays of exactly `len` valid elements for the batch.
    let deltas = unsafe { std::slice::from_raw_parts(deltas, len) };
    BATCH_CALLS.with(|cell| {
        cell.borrow_mut()
            .push((kind, handles.to_vec(), deltas.to_vec()));
    });
}

extern "C" fn isolating_batch(kind: u32, handles: *const u64, deltas: *const f64, len: u64) {
    let len = usize::try_from(len).unwrap_or(0);
    // SAFETY: the runtime passes arrays of exactly `len` valid elements for the batch.
    let handles = unsafe { std::slice::from_raw_parts(handles, len) };
    // SAFETY: the runtime passes arrays of exactly `len` valid elements for the batch.
    let deltas = unsafe { std::slice::from_raw_parts(deltas, len) };
    BATCH_CALLS.with(|cell| {
        cell.borrow_mut()
            .push((kind, handles.to_vec(), deltas.to_vec()));
    });
    for &handle in handles {
        if run_body(handle).is_err() {
            FAULTED.with(|cell| cell.borrow_mut().push(handle));
        }
    }
}

fn counting_callbacks() -> Callbacks {
    Callbacks {
        create_script: counting_create_script,
        awake: noop_handle,
        on_enable: noop_handle,
        start: noop_handle,
        fixed_update: noop_handle_delta,
        update: counting_update,
        late_update: noop_handle_delta,
        on_disable: noop_handle,
        destroy: noop_handle,
        collision: noop_collision,
        trigger: noop_collision,
        ..Callbacks::default()
    }
}

fn isolating_callbacks() -> Callbacks {
    Callbacks {
        create_script: counting_create_script,
        awake: noop_handle,
        on_enable: noop_handle,
        start: noop_handle,
        fixed_update: noop_handle_delta,
        update: isolating_update,
        late_update: noop_handle_delta,
        on_disable: noop_handle,
        destroy: noop_handle,
        collision: noop_collision,
        trigger: noop_collision,
        ..Callbacks::default()
    }
}

fn runtime_with_scripts(
    callbacks: Callbacks,
    count: usize,
) -> Result<(Runtime, Vec<u64>), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(EMPTY_SCENE_FIXTURE))?;
    runtime.set_callbacks(callbacks);
    let mut handles = Vec::new();
    for index in 0..count {
        let entity = runtime.create_runtime_object(&format!("BatchScript{index}"))?;
        runtime.attach_runtime_script(entity, 0x1000 + index as u64, 0x2000)?;
        let offset = 1000_u64.saturating_mul(index as u64 + 1);
        handles.push(entity.as_u64().saturating_add(offset));
    }
    Ok((runtime, handles))
}

#[test]
fn batch_dispatch_crosses_once_per_phase_in_handle_order() -> Result<(), Box<dyn Error>> {
    reset_state();
    let (mut runtime, handles) = runtime_with_scripts(counting_callbacks(), 3)?;
    runtime.set_script_batch_dispatch(counting_batch);

    runtime.tick(0.02)?;
    assert!(
        take_batch_calls().is_empty(),
        "first tick only initializes scripts"
    );

    runtime.tick(0.02)?;
    let update_calls = batch_calls_for(PHASE_UPDATE);
    assert_eq!(
        update_calls.len(),
        1,
        "exactly one managed crossing per Update phase"
    );
    let late_calls = batch_calls_for(PHASE_LATE_UPDATE);
    assert_eq!(
        late_calls.len(),
        1,
        "exactly one managed crossing per LateUpdate phase"
    );
    let fixed_calls = batch_calls_for(PHASE_FIXED_UPDATE);
    assert!(
        !fixed_calls.is_empty(),
        "fixed phase reaches the batch callback"
    );
    for (_, batch_handles, deltas) in update_calls.iter().chain(late_calls.iter()) {
        assert_eq!(batch_handles, &handles, "handle order preserved");
        assert!(
            deltas.iter().all(|delta| (*delta - 0.02).abs() < 1e-12),
            "per-script deltas carry the scaled frame delta"
        );
    }
    for (_, batch_handles, _) in fixed_calls {
        assert_eq!(batch_handles, handles, "handle order preserved");
    }
    assert_eq!(
        PER_SCRIPT_UPDATE_CALLS.with(|cell| cell.borrow().len()),
        0,
        "batch dispatch replaces per-script update crossings"
    );

    runtime.tick(0.02)?;
    assert_eq!(batch_calls_for(PHASE_UPDATE).len(), 2);
    assert_eq!(batch_calls_for(PHASE_LATE_UPDATE).len(), 2);
    Ok(())
}

#[test]
fn batch_dispatch_keeps_per_script_fault_attribution() -> Result<(), Box<dyn Error>> {
    reset_state();
    let (mut runtime, handles) = runtime_with_scripts(isolating_callbacks(), 3)?;
    let failing = handles[1];
    set_faulting(failing);
    runtime.set_script_batch_dispatch(isolating_batch);

    runtime.tick(0.02)?;
    DISPATCHED.with(|cell| cell.borrow_mut().clear());
    FAULTED.with(|cell| cell.borrow_mut().clear());
    runtime.tick(0.02)?;

    let mut expected = handles.clone();
    expected.retain(|handle| *handle != failing);
    let update_calls = batch_calls_for(PHASE_UPDATE);
    assert_eq!(update_calls.len(), 1);
    assert_eq!(update_calls[0].1, handles, "batch still lists every handle");
    let dispatched = dispatched();
    let phase_count = dispatched.len() / expected.len().max(1);
    for chunk in dispatched.chunks(expected.len()) {
        assert_eq!(
            chunk,
            expected.as_slice(),
            "scripts after the fault still run inside the same crossing"
        );
    }
    assert_eq!(
        faulted(),
        vec![failing; phase_count],
        "the failing script is identified by handle in every phase"
    );
    Ok(())
}

#[test]
fn per_script_path_attributes_faults_identically() -> Result<(), Box<dyn Error>> {
    reset_state();
    let (mut runtime, handles) = runtime_with_scripts(isolating_callbacks(), 3)?;
    let failing = handles[1];
    set_faulting(failing);

    runtime.tick(0.02)?;
    runtime.tick(0.02)?;

    let mut expected = handles.clone();
    expected.retain(|handle| *handle != failing);
    assert_eq!(dispatched(), expected);
    assert_eq!(faulted(), vec![failing]);
    assert!(
        take_batch_calls().is_empty(),
        "no batch crossing without a registered batch callback"
    );
    Ok(())
}

#[test]
fn batch_dispatch_with_no_scripts_crosses_zero_times() -> Result<(), Box<dyn Error>> {
    reset_state();
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(EMPTY_SCENE_FIXTURE))?;
    runtime.set_callbacks(counting_callbacks());
    runtime.set_script_batch_dispatch(counting_batch);

    runtime.tick(0.02)?;
    runtime.tick(0.02)?;
    runtime.tick(0.02)?;

    assert!(
        take_batch_calls().is_empty(),
        "an empty script list performs no managed crossings"
    );
    Ok(())
}
