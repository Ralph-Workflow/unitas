use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

use crossbeam_deque::{Injector, Steal, Stealer, Worker};

type Job = Box<dyn FnOnce() + Send + 'static>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JobHandle(u64);

impl JobHandle {
    pub const INVALID: Self = Self(0);

    pub fn is_valid(self) -> bool {
        self.0 != 0
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }

    pub fn from_u64(value: u64) -> Self {
        Self(value)
    }
}

struct RecordState {
    completed: bool,
    enqueued: bool,
    pending: usize,
    dependents: Vec<u64>,
    payload: Option<Job>,
    group: Option<u64>,
    group_remaining: usize,
}

struct Record {
    state: Mutex<RecordState>,
    signal: Condvar,
}

impl Record {
    fn leaf(payload: Job, pending: usize, group: Option<u64>) -> Self {
        Self {
            state: Mutex::new(RecordState {
                completed: false,
                enqueued: false,
                pending,
                dependents: Vec::new(),
                payload: Some(payload),
                group,
                group_remaining: 0,
            }),
            signal: Condvar::new(),
        }
    }

    fn group(remaining: usize) -> Self {
        Self {
            state: Mutex::new(RecordState {
                completed: false,
                enqueued: false,
                pending: 0,
                dependents: Vec::new(),
                payload: None,
                group: None,
                group_remaining: remaining,
            }),
            signal: Condvar::new(),
        }
    }
}

struct Inner {
    injector: Injector<u64>,
    stealers: Vec<Stealer<u64>>,
    records: Mutex<HashMap<u64, Arc<Record>>>,
    next_id: AtomicU64,
    queued: AtomicUsize,
    idle_lock: Mutex<()>,
    idle_signal: Condvar,
    shutdown: AtomicBool,
    live: AtomicUsize,
    active: AtomicUsize,
}

pub struct JobSystem {
    inner: Arc<Inner>,
    workers: Mutex<Vec<JoinHandle<()>>>,
}

impl JobSystem {
    pub fn new(workers: usize) -> Self {
        let count = workers.max(1);
        let locals: Vec<Worker<u64>> = (0..count).map(|_| Worker::new_fifo()).collect();
        let stealers: Vec<Stealer<u64>> = locals.iter().map(Worker::stealer).collect();
        let inner = Arc::new(Inner {
            injector: Injector::new(),
            stealers,
            records: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            queued: AtomicUsize::new(0),
            idle_lock: Mutex::new(()),
            idle_signal: Condvar::new(),
            shutdown: AtomicBool::new(false),
            live: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
        });
        let mut handles = Vec::with_capacity(count);
        for (index, local) in locals.into_iter().enumerate() {
            let shared = Arc::clone(&inner);
            handles.push(thread::spawn(move || worker_main(shared, local, index)));
        }
        Self {
            inner,
            workers: Mutex::new(handles),
        }
    }

    pub fn schedule<F>(&self, job: F, deps: &[JobHandle]) -> JobHandle
    where
        F: FnOnce() + Send + 'static,
    {
        let inner = &self.inner;
        let Some(dep_records) = resolve_dependencies(inner, deps) else {
            return JobHandle::INVALID;
        };
        let id = inner.next_id.fetch_add(1, Ordering::Relaxed);
        let record = Arc::new(Record::leaf(Box::new(job), dep_records.len(), None));
        insert(inner, id, &record);
        inner.live.fetch_add(1, Ordering::AcqRel);
        attach(inner, id, &record, &dep_records);
        JobHandle(id)
    }

    pub fn schedule_parallel_for<F>(
        &self,
        length: i64,
        batch: i64,
        job: F,
        deps: &[JobHandle],
    ) -> JobHandle
    where
        F: Fn(i64, i64) + Send + Sync + 'static,
    {
        let inner = &self.inner;
        let Some(dep_records) = resolve_dependencies(inner, deps) else {
            return JobHandle::INVALID;
        };
        let group_id = inner.next_id.fetch_add(1, Ordering::Relaxed);
        let batches = batch_count(length, batch);
        let group = Arc::new(Record::group(batches));
        insert(inner, group_id, &group);
        if batches == 0 {
            lock(&group.state).completed = true;
            return JobHandle(group_id);
        }
        let shared = Arc::new(job);
        let mut start = 0i64;
        for _ in 0..batches {
            let count = if batch <= 0 {
                length
            } else {
                batch.min(length - start)
            };
            let child_id = inner.next_id.fetch_add(1, Ordering::Relaxed);
            let child_job = Arc::clone(&shared);
            let payload: Job = Box::new(move || child_job(start, count));
            let child = Arc::new(Record::leaf(payload, dep_records.len(), Some(group_id)));
            insert(inner, child_id, &child);
            inner.live.fetch_add(1, Ordering::AcqRel);
            attach(inner, child_id, &child, &dep_records);
            start += count;
        }
        JobHandle(group_id)
    }

    pub fn complete(&self, handle: JobHandle) {
        let Some(record) = lookup(&self.inner, handle.as_u64()) else {
            return;
        };
        let mut state = lock(&record.state);
        while !state.completed {
            state = match record.signal.wait(state) {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
        }
    }

    pub fn is_complete(&self, handle: JobHandle) -> bool {
        match lookup(&self.inner, handle.as_u64()) {
            Some(record) => lock(&record.state).completed,
            None => false,
        }
    }

    pub fn active_workers(&self) -> usize {
        self.inner.active.load(Ordering::Acquire)
    }

    pub fn shutdown(&mut self) {
        self.inner.shutdown.store(true, Ordering::Release);
        notify(&self.inner);
        let mut workers = lock(&self.workers);
        for handle in workers.drain(..) {
            let _joined = handle.join().is_ok();
        }
    }
}

impl Drop for JobSystem {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn lookup(inner: &Inner, id: u64) -> Option<Arc<Record>> {
    lock(&inner.records).get(&id).cloned()
}

fn insert(inner: &Inner, id: u64, record: &Arc<Record>) {
    lock(&inner.records).insert(id, Arc::clone(record));
}

fn resolve_dependencies(inner: &Inner, deps: &[JobHandle]) -> Option<Vec<Arc<Record>>> {
    let mut resolved = Vec::with_capacity(deps.len());
    for handle in deps {
        if !handle.is_valid() {
            continue;
        }
        resolved.push(lookup(inner, handle.as_u64())?);
    }
    Some(resolved)
}

fn batch_count(length: i64, batch: i64) -> usize {
    if length <= 0 {
        return 0;
    }
    if batch <= 0 {
        return 1;
    }
    let batches = length / batch + i64::from(length % batch != 0);
    usize::try_from(batches).unwrap_or(usize::MAX)
}

fn attach(inner: &Inner, id: u64, record: &Arc<Record>, deps: &[Arc<Record>]) {
    for dep in deps {
        let completed = {
            let mut state = lock(&dep.state);
            if state.completed {
                true
            } else {
                state.dependents.push(id);
                false
            }
        };
        if completed {
            release(inner, id, record);
        }
    }
    if deps.is_empty() {
        mark_ready(inner, id, record);
    }
}

fn release(inner: &Inner, id: u64, record: &Arc<Record>) {
    let ready = {
        let mut state = lock(&record.state);
        state.pending = state.pending.saturating_sub(1);
        if state.pending == 0 && !state.enqueued {
            state.enqueued = true;
            true
        } else {
            false
        }
    };
    if ready {
        enqueue(inner, id);
    }
}

fn mark_ready(inner: &Inner, id: u64, record: &Arc<Record>) {
    let ready = {
        let mut state = lock(&record.state);
        if state.enqueued {
            false
        } else {
            state.enqueued = true;
            true
        }
    };
    if ready {
        enqueue(inner, id);
    }
}

fn enqueue(inner: &Inner, id: u64) {
    inner.queued.fetch_add(1, Ordering::AcqRel);
    inner.injector.push(id);
    notify(inner);
}

fn notify(inner: &Inner) {
    let _guard = lock(&inner.idle_lock);
    inner.idle_signal.notify_all();
}

fn wait_for_work(inner: &Inner) {
    let mut guard = lock(&inner.idle_lock);
    while inner.queued.load(Ordering::Acquire) == 0
        && !(inner.shutdown.load(Ordering::Acquire) && inner.live.load(Ordering::Acquire) == 0)
    {
        guard = match inner.idle_signal.wait(guard) {
            Ok(waiting) => waiting,
            Err(poisoned) => poisoned.into_inner(),
        };
    }
}

fn worker_main(inner: Arc<Inner>, local: Worker<u64>, index: usize) {
    inner.active.fetch_add(1, Ordering::AcqRel);
    loop {
        if let Some(id) = take_next(&inner, &local, index) {
            run_job(&inner, id);
            continue;
        }
        if inner.shutdown.load(Ordering::Acquire) && inner.live.load(Ordering::Acquire) == 0 {
            break;
        }
        wait_for_work(&inner);
    }
    inner.active.fetch_sub(1, Ordering::AcqRel);
}

fn take_next(inner: &Inner, local: &Worker<u64>, index: usize) -> Option<u64> {
    loop {
        if let Some(id) = local.pop() {
            return Some(id);
        }
        for (offset, stealer) in inner.stealers.iter().enumerate() {
            if offset == index {
                continue;
            }
            if let Some(id) = steal(stealer) {
                return Some(id);
            }
        }
        match inner.injector.steal_batch_and_pop(local) {
            Steal::Success(id) => return Some(id),
            Steal::Retry => {}
            Steal::Empty => return None,
        }
    }
}

fn steal(stealer: &Stealer<u64>) -> Option<u64> {
    loop {
        match stealer.steal() {
            Steal::Success(id) => return Some(id),
            Steal::Retry => {}
            Steal::Empty => return None,
        }
    }
}

fn run_job(inner: &Inner, id: u64) {
    inner.queued.fetch_sub(1, Ordering::AcqRel);
    let Some(record) = lookup(inner, id) else {
        return;
    };
    let payload = lock(&record.state).payload.take();
    if let Some(job) = payload {
        let outcome = catch_unwind(AssertUnwindSafe(job));
        let _failed = outcome.is_err();
    }
    finish(inner, &record);
}

fn finish(inner: &Inner, record: &Arc<Record>) {
    let (dependents, group) = {
        let mut state = lock(&record.state);
        state.completed = true;
        (state.dependents.drain(..).collect::<Vec<_>>(), state.group)
    };
    record.signal.notify_all();
    for dependent in dependents {
        if let Some(dependent_record) = lookup(inner, dependent) {
            release(inner, dependent, &dependent_record);
        }
    }
    if let Some(group_id) = group {
        finish_group(inner, group_id);
    }
    inner.live.fetch_sub(1, Ordering::AcqRel);
    notify(inner);
}

fn finish_group(inner: &Inner, group_id: u64) {
    let Some(record) = lookup(inner, group_id) else {
        return;
    };
    let dependents = {
        let mut state = lock(&record.state);
        state.group_remaining = state.group_remaining.saturating_sub(1);
        if state.group_remaining > 0 {
            return;
        }
        state.completed = true;
        state.dependents.drain(..).collect::<Vec<_>>()
    };
    record.signal.notify_all();
    for dependent in dependents {
        if let Some(dependent_record) = lookup(inner, dependent) {
            release(inner, dependent, &dependent_record);
        }
    }
    notify(inner);
}
