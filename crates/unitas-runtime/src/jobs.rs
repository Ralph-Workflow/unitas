use std::sync::OnceLock;

use unitas_jobs::JobSystem;

static JOB_SYSTEM: OnceLock<JobSystem> = OnceLock::new();

pub fn job_system() -> &'static JobSystem {
    JOB_SYSTEM.get_or_init(|| JobSystem::new(worker_count()))
}

fn worker_count() -> usize {
    std::thread::available_parallelism().map_or(1, |count| count.get())
}
