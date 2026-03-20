use std::process;
use sysinfo::System;
use std::ffi::OsStr;

pub fn check_already_running() -> bool {
    let mut s = System::new_all();
    s.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let current_pid = process::id();
    
    let running_processes = s.processes_by_exact_name(OsStr::new("rust-governor.exe"));
    
    let mut count = 0;
    for process in running_processes {
        if process.pid().as_u32() != current_pid {
            count += 1;
        }
    }
    count > 0
}