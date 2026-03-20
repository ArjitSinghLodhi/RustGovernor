use clap::Parser;
use sysinfo::System;
use std::thread::{self};
use std::time::Duration;
use anyhow::{Result, anyhow};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, SYSTEM_POWER_STATUS,
    PowerSetActiveScheme, PowerGetActiveScheme,
    PowerWriteACValueIndex, PowerWriteDCValueIndex,
};
use windows::core::GUID;
use std::fs::File;
use std::io::{Write, BufReader, BufRead};
use std::path::Path;

mod planhandler;
mod setup;
mod service_handler;
mod monitor_handling;
#[derive(Parser, Debug)]
#[command(
    author, 
    version, 
    about, 
    long_about = None,
    arg_required_else_help = true // Add this line!
)]
struct Args {
    #[arg(short, long, help = "Installs RustGovernor")] install: bool,
    #[arg(short, long, help = "Removes RustGovernor")] uninstall: bool,
    #[arg(short, long, help = "Runs RustGovernor with no verbose output, used for service function")] run: bool,
    #[arg(short, long, help = "Enable detailed monitoring and logging by reading data")] monitor: bool,
}
pub const GUID_PROCESSOR_SETTINGS_SUBGROUP: GUID = GUID::from_u128(0x54533251_82be_4824_96c1_47b60b740d00);
pub const GUID_PERFBOOSTMODE: GUID = GUID::from_u128(0xbe337238_0d82_4146_a960_4f3749d470c7);
pub const GUID_PROCPERFMAX: GUID = GUID::from_u128(0xbc5038f7_23e0_4960_96da_33abaf5935ec);
pub const GUID_EPP: GUID = GUID::from_u128(0x36687f9e_e3a5_4dbf_b112_83092b2d33a4);
pub const GUID_COOLING: GUID = GUID::from_u128(0x94d3a615_a899_4ac5_ae2b_e4d8f634367f);

struct Config {
    ac_max: Vec<(f32, u32)>,
    ac_turbo: Vec<(f32, u32)>,
    ac_epp: Vec<(f32, u32)>,
    ac_cooling: u32,
    dc_max_cap: u32,
    dc_max: Vec<(f32, u32)>,
    dc_epp: Vec<(f32, u32)>,
    dc_cooling: u32,
}
impl Config {
    fn default_content() -> &'static str {
    "ac_15_max=80
ac_20_max=100
ac_30_max=100
ac_40_max=100
ac_50_max=100
ac_60_max=100
ac_70_max=100
ac_80_max=100
ac_90_max=100
ac_100_max=100
ac_15_turbo=0
ac_50_turbo=0
ac_100_turbo=2
ac_1_epp=100
ac_5_epp=90
ac_10_epp=80
ac_15_epp=70
ac_20_epp=60
ac_40_epp=50
ac_60_epp=33
ac_80_epp=20
ac_100_epp=0
ac_cooling_threshold=45
dc_max_cap=70
dc_15_max=50
dc_40_max=60
dc_60_max=70
dc_100_max=70
dc_1_epp=100
dc_15_epp=90
dc_40_epp=80
dc_60_epp=70
dc_100_epp=60
dc_cooling_threshold=60"
    }

    fn load() -> Result<Self> {
        let exe_path = std::env::current_exe()?;
        if let Some(exe_dir) = exe_path.parent() {
            std::env::set_current_dir(exe_dir)?;
        }
        let path = "config.txt";
        if !Path::new(path).exists() {
            let mut f = File::create(path)?;
            f.write_all(Self::default_content().as_bytes())?;
        }

        let mut config = Self { ac_max: vec![], ac_turbo: vec![], ac_epp: vec![], ac_cooling: 45, dc_max_cap: 70, dc_max: vec![], dc_epp: vec![], dc_cooling: 60 };
        let reader = BufReader::new(File::open(path)?);
        for (idx, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() || line.starts_with('#') { continue; }
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() != 2 { return Err(anyhow!("Crash: Invalid format on line {}", idx + 1)); }
            let key = parts[0].trim();
            let val: u32 = parts[1].trim().parse().map_err(|_| anyhow!("Crash: Invalid number on line {} for key {}", idx + 1, key))?;
            
            if key.starts_with("ac_") && key.ends_with("_max") {
                let load: f32 = key[3..key.len()-4].parse().unwrap_or(0.0);
                config.ac_max.push((load, val));
            } else if key.starts_with("ac_") && key.ends_with("_turbo") {
                let load: f32 = key[3..key.len()-6].parse().unwrap_or(0.0);
                config.ac_turbo.push((load, val));
            } else if key.starts_with("ac_") && key.ends_with("_epp") {
                let load: f32 = key[3..key.len()-4].parse().unwrap_or(0.0);
                config.ac_epp.push((load, val));
            } else if key == "ac_cooling_threshold" {
                config.ac_cooling = val;
            } else if key == "dc_max_cap" {
                config.dc_max_cap = val;
            } else if key.starts_with("dc_") && key.ends_with("_max") {
                let load: f32 = key[3..key.len()-4].parse().unwrap_or(0.0);
                config.dc_max.push((load, val));
            } else if key.starts_with("dc_") && key.ends_with("_epp") {
                let load: f32 = key[3..key.len()-4].parse().unwrap_or(0.0);
                config.dc_epp.push((load, val));
            } else if key == "dc_cooling_threshold" {
                config.dc_cooling = val;
            }
        }
        config.ac_max.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        config.ac_turbo.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        config.ac_epp.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        config.dc_max.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        config.dc_epp.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        config.dc_cooling;
        config.ac_cooling;
        Ok(config)
    }
}

struct PowerManager;
impl PowerManager {
    fn get_active_guid() -> Result<GUID> {
        unsafe {
            let mut ptr: *mut GUID = std::ptr::null_mut();
            if PowerGetActiveScheme(None, &mut ptr) == windows::Win32::Foundation::WIN32_ERROR(0) && !ptr.is_null() { 
                let guid = *ptr;
                let _ = windows::Win32::Foundation::LocalFree(Some(windows::Win32::Foundation::HLOCAL(ptr as *mut _)));
            Ok(guid) 
        }
        else { Err(anyhow!("Active scheme fail")) 
        }
        }
    }
    fn get_ac_status() -> Result<bool> {
        unsafe {
            let mut s = SYSTEM_POWER_STATUS::default();
            if GetSystemPowerStatus(&mut s).is_ok() { Ok(s.ACLineStatus == 1) } 
            else { Err(anyhow!("Power status fail")) }
        }
    }
    fn ensure_custom_plan() -> Result<GUID> {
        let target_name = String::from("RustGovernorPlan");
        let rust_plan: GUID = unsafe { planhandler::power_plan(&target_name).expect("failed") };
        let _ = unsafe { PowerSetActiveScheme(None, Some(&rust_plan))};
        return Ok(rust_plan);
    }
    fn update_setting(setting: &GUID, val: u32, is_ac: bool, current_guid: &GUID) -> Result<()> {
        unsafe {
             let res = if is_ac { 
                 PowerWriteACValueIndex(None, &*current_guid, Some(&GUID_PROCESSOR_SETTINGS_SUBGROUP), Some(setting), val) 
            } else { 
                windows::Win32::Foundation::WIN32_ERROR(PowerWriteDCValueIndex(None, &*current_guid, Some(&GUID_PROCESSOR_SETTINGS_SUBGROUP), Some(setting), val)) 
             };
    
           if res == windows::Win32::Foundation::WIN32_ERROR(0) { Ok(()) } else { Err(anyhow!("Write failed with code: {:?}", res)) }
        }
    }
    }

struct GovernorState {
    avg_load: f32,
    history: Vec<f32>,
    max_history: usize,
    last_ac_status: Option<bool>,
    last_ac_boost: Option<u32>,
    last_ac_max: Option<u32>,
    last_ac_epp: Option<u32>,
    last_ac_cooling: Option<u32>,
    last_dc_boost: Option<u32>,
    last_dc_max: Option<u32>,
    last_dc_epp: Option<u32>,
    last_dc_cooling: Option<u32>,
}

impl GovernorState {
    fn new() -> Self {
        Self { avg_load: 0.0, history: Vec::new(), max_history: 10, last_ac_status: None, last_ac_boost: None, last_ac_max: None, last_ac_epp: None, last_ac_cooling: None, last_dc_boost: None, last_dc_max: None, last_dc_epp: None, last_dc_cooling: None }
    }
    fn add_load(&mut self, load: f32) {
        self.history.push(load);
        if self.history.len() > self.max_history { self.history.remove(0); }
        self.avg_load = self.history.iter().sum::<f32>() / self.history.len() as f32;
    }
}

fn apply_hardware_setting(state: &mut GovernorState, t_max: u32, t_turbo: u32, t_epp: u32, t_cooling: u32, is_ac: bool, changed: bool) -> Result<()> {
    let current_guid = PowerManager::get_active_guid()?;
    if is_ac {
        if state.last_ac_boost != Some(t_turbo) || changed {
            match PowerManager::update_setting(&GUID_PERFBOOSTMODE, t_turbo, true, &current_guid) {
                ::std::result::Result::Ok(_) => { state.last_ac_boost = Some(t_turbo); },
                ::std::result::Result::Err(e) => {eprintln!("[ERROR] Failed to set Turbo: {}", e); }
            }
        }
        if state.last_ac_max != Some(t_max) || changed {
            match PowerManager::update_setting(&GUID_PROCPERFMAX, t_max, true, &current_guid) {
                ::std::result::Result::Ok(_) => { state.last_ac_max = Some(t_max); },
                ::std::result::Result::Err(e) => {eprintln!("[ERROR] Failed to set Max CPU Percentage: {}", e); }
            }
        }
        if state.last_ac_epp != Some(t_epp) || changed {
            match PowerManager::update_setting(&GUID_EPP, t_epp, true, &current_guid) {
                ::std::result::Result::Ok(_) => { state.last_ac_epp = Some(t_epp); },
                ::std::result::Result::Err(e) => { eprintln!("[ERROR] Failed to set EPP: {}", e); }
            }
        }
        if state.last_ac_cooling != Some(t_cooling) || changed {
            match PowerManager::update_setting(&GUID_COOLING, t_cooling, true, &current_guid) {
                ::std::result::Result::Ok(_) => { state.last_ac_cooling = Some(t_cooling); },
                ::std::result::Result::Err(e) => {eprintln!("[ERROR] Failed to set Cooling mode: {}", e); }
            }
        }
    } else {
        if state.last_dc_boost != Some(t_turbo) || changed {
            match PowerManager::update_setting(&GUID_PERFBOOSTMODE, t_turbo, false, &current_guid) {
                ::std::result::Result::Ok(_) => { state.last_dc_boost = Some(t_turbo); },
                ::std::result::Result::Err(e) => {eprintln!("[ERROR] Failed to set Turbo: {}", e); }
            }
        }
        if state.last_dc_max != Some(t_max) || changed {
            match PowerManager::update_setting(&GUID_PROCPERFMAX, t_max, false, &current_guid) {
                ::std::result::Result::Ok(_) => { state.last_dc_max = Some(t_max); },
                ::std::result::Result::Err(e) => {eprintln!("[ERROR] Failed to set Max CPU Percentage: {}", e); }
            }
        }
        if state.last_dc_epp != Some(t_epp) || changed {
            match PowerManager::update_setting(&GUID_EPP, t_epp, false, &current_guid) {
                ::std::result::Result::Ok(_) => { state.last_dc_epp = Some(t_epp); },
                ::std::result::Result::Err(e) => {eprintln!("[ERROR] Failed to set EPP: {}", e); }
            }
        }
         if state.last_dc_cooling != Some(t_cooling) || changed {
            match PowerManager::update_setting(&GUID_COOLING, t_cooling, false, &current_guid) {
                ::std::result::Result::Ok(_) => { state.last_dc_cooling = Some(t_cooling); },
                ::std::result::Result::Err(e) => {eprintln!("[ERROR] Failed to set Cooling mode: {}", e); }
            }
        }
}
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::try_parse().unwrap_or_else(|e| e.exit());

    let flags = [args.install, args.uninstall, args.run, args.monitor];
    if flags.iter().filter(|&&f| f).count() > 1 {
        eprintln!("Error: Please provide only one flag at a time.");
        std::process::exit(1);
    }


    if args.install {
        setup::self_install()?;
        return Ok(())
    }
    if args.uninstall {
        setup::uninstall()?;
        return Ok(())
    }
    let monitor_mode = args.monitor;
    
    if monitor_mode {
        println!("--------------------------------------------------");
        println!("RustGovernor v0.2.0 Initializing...");
        PowerManager::ensure_custom_plan()?;
        println!("Plan Activated | Rate: 1s | History: 10s Window");
        println!("--------------------------------------------------");
    } else {
        PowerManager::ensure_custom_plan()?;
    }
    if args.monitor {
        return monitor_handling::monitor_loop();
    }
    
    if args.run {
        if service_handler::check_already_running() {
            eprintln!("Rust Governor is already running in the background.");
            std::process::exit(1);
        }
       unsafe {let _ = windows::Win32::System::Console::FreeConsole();}
       let config = Config::load()?;
       let mut sys = System::new_all();
       let mut state = GovernorState::new();
       let mut rust_plan_guid = PowerManager::ensure_custom_plan()?;
    loop {
        let active_guid = PowerManager::get_active_guid()?;
        if active_guid != rust_plan_guid {
            if unsafe {PowerSetActiveScheme(None, Some(&rust_plan_guid))} != windows::Win32::Foundation::WIN32_ERROR(0) {
                if let Ok(new_guid) = PowerManager::ensure_custom_plan() {
                    rust_plan_guid = new_guid;
                }
            }
        }
        sys.refresh_cpu_usage();
        let cpus = sys.cpus();
        state.add_load(if cpus.is_empty() { 0.0 } else { cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32 });
        let is_ac = PowerManager::get_ac_status().unwrap_or(true);
        let changed = state.last_ac_status != Some(is_ac);
        state.last_ac_status = Some(is_ac);

        let (mut t_max, mut t_turbo) = if is_ac { (100, 0) } else { (config.dc_max_cap, 0) };
        let mut t_epp: u32 = if is_ac {100} else {config.dc_max_cap};
        let mut t_cooling: u32 = 0;
        if is_ac {
            for (threshold, val) in &config.ac_max { if state.avg_load <= *threshold { t_max = *val; break; } }
            for (threshold, val) in &config.ac_turbo { if state.avg_load <= *threshold { t_turbo = *val; break; } }
            for (threshold, val) in &config.ac_epp { if state.avg_load <= *threshold { t_epp = *val; break; } }
            if state.avg_load <= (*&config.ac_cooling as f32) { t_cooling = 0} else if state.avg_load >= (*&config.ac_cooling as f32) {t_cooling = 1};
        } else {
            for (threshold, val) in &config.dc_max { if state.avg_load <= *threshold { t_max = (*val).min(config.dc_max_cap); break; } }
            for (threshold, val) in &config.dc_epp { if state.avg_load <= *threshold { t_epp = *val; break; } }
             if state.avg_load <= (*&config.dc_cooling as f32) { t_cooling = 0} else if state.avg_load >= (*&config.dc_cooling as f32) {t_cooling = 1};
        }

        let _ = apply_hardware_setting(&mut state, t_max, t_turbo, t_epp, t_cooling, is_ac, changed);
        thread::sleep(Duration::from_secs(1));
    }
}   Ok(())
}
