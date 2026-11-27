use super::{
    JobHandle, Path, c_void, job_system, read_path_buffer, with_runtime_state,
    write_string_to_path_buffer,
};

ffi_export! {
pub extern "C" fn unitas_runtime_script_index_build(handle: u64, address: u64, length: u64) -> i32 {
    let len = match usize::try_from(length) {
        Ok(l) => l,
        Err(_) => return -2,
    };
    match with_runtime_state(handle, |state| {
        let path_str = match read_path_buffer(state, address, len) {
            Ok(text) => text,
            Err(code) => return code,
        };
        let index = match crate::scripting::ProjectScriptIndex::build(Path::new(&path_str)) {
            Ok(idx) => idx,
            Err(err) => {
                state.set_error(&err.to_string());
                return -4;
            }
        };
        let manifest = index.to_manifest_tsv();
        write_string_to_path_buffer(
            state,
            &manifest,
            "script index manifest exceeds supported length",
        )
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_resources_scan(handle: u64, address: u64, length: u64) -> i32 {
    let len = match usize::try_from(length) {
        Ok(l) => l,
        Err(_) => return -2,
    };
    match with_runtime_state(handle, |state| {
        let path_str = match read_path_buffer(state, address, len) {
            Ok(text) => text,
            Err(code) => return code,
        };
        let index = match crate::resources::ResourcesIndex::scan(Path::new(&path_str)) {
            Ok(idx) => idx,
            Err(err) => {
                state.set_error(&err.to_string());
                return -4;
            }
        };
        let mut lines = Vec::new();
        for entry in index.iter() {
            let guid = entry.guid.as_deref().unwrap_or("");
            lines.push(format!(
                "{}\t{}\t{}\t{}",
                entry.canonical_key,
                entry.resource_path,
                entry.file_path.display(),
                guid
            ));
        }
        let manifest = lines.join("\n");
        write_string_to_path_buffer(
            state,
            &manifest,
            "resources manifest exceeds supported length",
        )
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

fn job_dependencies(dependency: u64) -> Vec<JobHandle> {
    if dependency == 0 {
        return Vec::new();
    }
    vec![JobHandle::from_u64(dependency)]
}

ffi_export! {
pub extern "C" fn unitas_runtime_jobs_schedule_parallel(
    cb: extern "C" fn(*mut c_void, i64, i64),
    ctx: *mut c_void,
    length: i64,
    batch: i64,
    dependency: u64,
) -> u64 {
    let address = ctx as usize;
    let deps = job_dependencies(dependency);
    job_system()
        .schedule_parallel_for(
            length,
            batch,
            move |start, count| {
                cb(address as *mut c_void, start, count);
            },
            &deps,
        )
        .as_u64()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_jobs_schedule(
    cb: extern "C" fn(*mut c_void, i64, i64),
    ctx: *mut c_void,
    dependency: u64,
) -> u64 {
    let address = ctx as usize;
    let deps = job_dependencies(dependency);
    job_system()
        .schedule(
            move || {
                cb(address as *mut c_void, 0, 1);
            },
            &deps,
        )
        .as_u64()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_jobs_complete(handle: u64) {
    job_system().complete(JobHandle::from_u64(handle));
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_jobs_is_complete(handle: u64) -> u8 {
    u8::from(job_system().is_complete(JobHandle::from_u64(handle)))
}
}
