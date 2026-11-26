use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use unitas_jobs::{JobHandle, JobSystem};

fn system(workers: usize) -> JobSystem {
    JobSystem::new(workers)
}

#[test]
fn every_scheduled_job_runs_exactly_once() {
    let pool = system(4);
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..128 {
        let counter = Arc::clone(&counter);
        handles.push(pool.schedule(
            move || {
                counter.fetch_add(1, Ordering::SeqCst);
            },
            &[],
        ));
    }
    for handle in &handles {
        pool.complete(*handle);
    }
    assert_eq!(counter.load(Ordering::SeqCst), 128);
    assert!(handles.iter().all(|handle| pool.is_complete(*handle)));
}

#[test]
fn dependent_job_observes_its_dependency_completed() {
    let pool = system(4);
    let dependency_done = Arc::new(AtomicBool::new(false));
    let violations = Arc::new(AtomicUsize::new(0));
    let flag = Arc::clone(&dependency_done);
    let dependency = pool.schedule(
        move || {
            flag.store(true, Ordering::SeqCst);
        },
        &[],
    );
    let observed = Arc::clone(&dependency_done);
    let counter = Arc::clone(&violations);
    let dependent = pool.schedule(
        move || {
            if !observed.load(Ordering::SeqCst) {
                counter.fetch_add(1, Ordering::SeqCst);
            }
        },
        &[dependency],
    );
    pool.complete(dependent);
    assert_eq!(violations.load(Ordering::SeqCst), 0);
    assert!(pool.is_complete(dependency));
}

#[test]
fn parallel_for_covers_the_range_exactly_once() {
    let pool = system(4);
    let seen = Arc::new(Mutex::new(Vec::<i64>::new()));
    let recorder = Arc::clone(&seen);
    let handle = pool.schedule_parallel_for(
        10,
        3,
        move |start, count| {
            let mut entries = recorder.lock().unwrap_or_else(|error| error.into_inner());
            for index in start..start + count {
                entries.push(index);
            }
        },
        &[],
    );
    pool.complete(handle);
    let entries = seen.lock().unwrap_or_else(|error| error.into_inner());
    let mut sorted = entries.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, (0..10).collect::<Vec<i64>>());
}

#[test]
fn parallel_for_with_a_non_positive_batch_uses_one_batch() {
    let pool = system(2);
    let seen = Arc::new(Mutex::new(Vec::<(i64, i64)>::new()));
    let recorder = Arc::clone(&seen);
    let handle = pool.schedule_parallel_for(
        8,
        0,
        move |start, count| {
            let mut entries = recorder.lock().unwrap_or_else(|error| error.into_inner());
            entries.push((start, count));
        },
        &[],
    );
    pool.complete(handle);
    let entries = seen.lock().unwrap_or_else(|error| error.into_inner());
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0], (0, 8));
}

#[test]
fn parallel_for_over_an_empty_range_completes_without_a_callback() {
    let pool = system(2);
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let handle = pool.schedule_parallel_for(
        0,
        4,
        move |_start, _count| {
            counter.fetch_add(1, Ordering::SeqCst);
        },
        &[],
    );
    pool.complete(handle);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(pool.is_complete(handle));
}

#[test]
fn complete_waits_for_the_job_and_is_complete_reports_the_state() {
    let pool = system(2);
    let ran = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ran);
    let handle = pool.schedule(
        move || {
            flag.store(true, Ordering::SeqCst);
        },
        &[],
    );
    pool.complete(handle);
    assert!(ran.load(Ordering::SeqCst));
    assert!(pool.is_complete(handle));
    assert!(!pool.is_complete(JobHandle::from_u64(9_999)));
}

#[test]
fn a_completed_handle_remains_a_valid_dependency_within_its_window() {
    let pool = system(2);
    let dependency = pool.schedule(|| {}, &[]);
    pool.complete(dependency);
    let dependent = pool.schedule(|| {}, &[dependency]);
    assert!(dependent.is_valid());
    pool.complete(dependent);
    assert!(pool.is_complete(dependent));
}

#[test]
fn an_unknown_dependency_is_rejected() {
    let pool = system(2);
    let handle = pool.schedule(|| {}, &[JobHandle::from_u64(4_242)]);
    assert!(!handle.is_valid());
    assert_eq!(handle.as_u64(), 0);
}

#[test]
fn shutdown_joins_every_worker_thread() {
    let mut pool = system(3);
    let counter = Arc::new(AtomicUsize::new(0));
    let runs = Arc::clone(&counter);
    let handle = pool.schedule(
        move || {
            runs.fetch_add(1, Ordering::SeqCst);
        },
        &[],
    );
    pool.complete(handle);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    pool.shutdown();
    assert_eq!(pool.active_workers(), 0);
}
