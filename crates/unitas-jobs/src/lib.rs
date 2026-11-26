use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

use crossbeam_deque::{Injector, Steal, Stealer, Worker};

type Job = Box<dyn FnOnce() + Send + 'static>;

const COMPLETED_WINDOW_WORDS: usize = 1_024;

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
    pending_dependents: usize,
    dependents: Vec<u64>,
    dependencies: Vec<u64>,
    payload: Option<Job>,
    group: Option<u64>,
    group_remaining: usize,
}

struct Record {
    state: Mutex<RecordState>,
    signal: Condvar,
}

struct CompletedWindow {
    base: u64,
    bits: Vec<u64>,
}

impl CompletedWindow {
    fn new() -> Self {
        Self {
            base: 1,
            bits: vec![0; COMPLETED_WINDOW_WORDS],
        }
    }

    fn mark(&mut self, id: u64) {
        let capacity = COMPLETED_WINDOW_WORDS as u64 * 64;
        if id.saturating_sub(self.base) >= capacity {
            self.base = id.saturating_add(1).saturating_sub(capacity);
            self.bits.fill(0);
        }
        let index = id.saturating_sub(self.base) as usize;
        if index < COMPLETED_WINDOW_WORDS * 64 {
            self.bits[index / 64] |= 1u64 << (index % 64);
        }
    }

    fn contains(&self, id: u64) -> bool {
        let Some(offset) = id.checked_sub(self.base) else {
            return false;
        };
        let index = offset as usize;
        if index >= COMPLETED_WINDOW_WORDS * 64 {
            return false;
        }
        self.bits[index / 64] & (1u64 << (index % 64)) != 0
    }
}

impl Record {
    fn leaf(payload: Job, pending: usize, group: Option<u64>) -> Self {
        Self {
            state: Mutex::new(RecordState {
                completed: false,
                enqueued: false,
                pending,
                pending_dependents: 0,
                dependents: Vec::new(),
                dependencies: Vec::new(),
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
                pending_dependents: 0,
                dependents: Vec::new(),
                dependencies: Vec::new(),
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
    completed: Mutex<CompletedWindow>,
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
            completed: Mutex::new(CompletedWindow::new()),
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
            mark_completed(inner, group_id);
            retire_unreferenced(inner, group_id, &group);
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
        let id = handle.as_u64();
        match lookup(&self.inner, id) {
            Some(record) => lock(&record.state).completed,
            None => completed_in_window(&self.inner, id),
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

fn mark_completed(inner: &Inner, id: u64) {
    lock(&inner.completed).mark(id);
}

fn completed_in_window(inner: &Inner, id: u64) -> bool {
    lock(&inner.completed).contains(id)
}

fn resolve_dependencies(inner: &Inner, deps: &[JobHandle]) -> Option<Vec<(u64, Arc<Record>)>> {
    let mut pending = Vec::with_capacity(deps.len());
    for handle in deps {
        if !handle.is_valid() {
            continue;
        }
        let id = handle.as_u64();
        match lookup(inner, id) {
            Some(record) => {
                let completed = lock(&record.state).completed;
                if !completed {
                    pending.push((id, record));
                }
            }
            None => {
                if !completed_in_window(inner, id) {
                    return None;
                }
            }
        }
    }
    Some(pending)
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

fn attach(inner: &Inner, id: u64, record: &Arc<Record>, deps: &[(u64, Arc<Record>)]) {
    {
        let mut state = lock(&record.state);
        state.dependencies = deps.iter().map(|(dependency, _)| *dependency).collect();
    }
    for (_, dep) in deps {
        let completed = {
            let mut state = lock(&dep.state);
            state.pending_dependents += 1;
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
    finish(inner, id, &record);
}

fn finish(inner: &Inner, id: u64, record: &Arc<Record>) {
    let (dependents, group, dependencies) = {
        let mut state = lock(&record.state);
        state.completed = true;
        (
            state.dependents.drain(..).collect::<Vec<_>>(),
            state.group,
            std::mem::take(&mut state.dependencies),
        )
    };
    for dependent in dependents {
        if let Some(dependent_record) = lookup(inner, dependent) {
            release(inner, dependent, &dependent_record);
        }
    }
    for dependency in dependencies {
        release_dependency(inner, dependency);
    }
    mark_completed(inner, id);
    retire_unreferenced(inner, id, record);
    if let Some(group_id) = group {
        finish_group(inner, group_id);
    }
    record.signal.notify_all();
    inner.live.fetch_sub(1, Ordering::AcqRel);
    notify(inner);
}

fn release_dependency(inner: &Inner, id: u64) {
    let Some(record) = lookup(inner, id) else {
        return;
    };
    let retirable = {
        let mut state = lock(&record.state);
        state.pending_dependents = state.pending_dependents.saturating_sub(1);
        state.pending_dependents == 0 && state.completed
    };
    if retirable {
        lock(&inner.records).remove(&id);
    }
}

fn retire_unreferenced(inner: &Inner, id: u64, record: &Arc<Record>) {
    let unreferenced = lock(&record.state).pending_dependents == 0;
    if unreferenced {
        lock(&inner.records).remove(&id);
    }
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
    for dependent in dependents {
        if let Some(dependent_record) = lookup(inner, dependent) {
            release(inner, dependent, &dependent_record);
        }
    }
    mark_completed(inner, group_id);
    retire_unreferenced(inner, group_id, &record);
    record.signal.notify_all();
    notify(inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_parallel_for_batches_leave_no_record_behind() {
        let pool = JobSystem::new(2);
        let handle = pool.schedule_parallel_for(64, 8, |_, _| {}, &[]);
        pool.complete(handle);
        assert!(pool.is_complete(handle));
        assert_eq!(lock(&pool.inner.records).len(), 0);
    }

    #[test]
    fn a_scheduled_job_stays_resolvable_as_a_dependency() {
        let pool = JobSystem::new(2);
        let dependency = pool.schedule(|| {}, &[]);
        pool.complete(dependency);
        let dependent = pool.schedule(|| {}, &[dependency]);
        pool.complete(dependent);
        assert!(pool.is_complete(dependent));
    }

    #[test]
    fn completed_leaf_records_are_retired_so_the_record_map_stays_bounded() {
        let pool = JobSystem::new(4);
        let scheduled = 4_096;
        for _ in 0..scheduled {
            let dependency = pool.schedule(|| {}, &[]);
            let dependent = pool.schedule(|| {}, &[dependency]);
            pool.complete(dependent);
        }
        let records = lock(&pool.inner.records).len();
        assert_eq!(records, 0, "records map holds {records} entries");
    }

    #[test]
    fn a_completed_record_is_kept_while_a_pending_dependent_references_it() {
        let pool = JobSystem::new(2);
        let (sender, receiver) = std::sync::mpsc::channel();
        let dependency = pool.schedule(|| {}, &[]);
        let dependent = pool.schedule(
            move || {
                let _message = receiver.recv();
            },
            &[dependency],
        );
        pool.complete(dependency);
        assert_eq!(lock(&pool.inner.records).len(), 2);
        let _sent = sender.send(());
        pool.complete(dependent);
        assert_eq!(lock(&pool.inner.records).len(), 0);
    }
}
