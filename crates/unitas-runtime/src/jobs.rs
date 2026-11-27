use std::sync::{Arc, OnceLock};

use unitas_jobs::JobSystem;

static JOB_SYSTEM: OnceLock<Arc<JobSystem>> = OnceLock::new();

pub fn job_system() -> &'static JobSystem {
    JOB_SYSTEM
        .get_or_init(|| Arc::new(JobSystem::new(worker_count())))
        .as_ref()
}

pub fn shared_job_system() -> Arc<JobSystem> {
    Arc::clone(JOB_SYSTEM.get_or_init(|| Arc::new(JobSystem::new(worker_count()))))
}

fn worker_count() -> usize {
    std::thread::available_parallelism().map_or(1, |count| count.get())
}
