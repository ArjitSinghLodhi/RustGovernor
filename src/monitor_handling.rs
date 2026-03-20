use anyhow::{Result, anyhow};
use windows::core::GUID;
use windows::Win32::System::Power::{
    PowerReadACValueIndex, PowerReadDCValueIndex,
};
use windows::Win32::Foundation::WIN32_ERROR;
use crate::{GUID_PROCESSOR_SETTINGS_SUBGROUP, GUID_PROCPERFMAX, GUID_EPP, GUID_PERFBOOSTMODE, GUID_COOLING, PowerManager};
use std::thread;
use std::time::Duration;

fn read_win_setting(setting: &GUID, is_ac: bool, active_guid: &GUID) -> Result<u32> {
    unsafe {
        let mut val: u32 = 0;
        
        let res = if is_ac {
            PowerReadACValueIndex(
                None, 
                Some(active_guid as *const GUID), 
                Some(&GUID_PROCESSOR_SETTINGS_SUBGROUP), 
                Some(setting as *const GUID), 
                &mut val as *mut u32,
            )
        } else {
            WIN32_ERROR(PowerReadDCValueIndex(
                None, 
                Some(active_guid as *const GUID), 
                Some(&GUID_PROCESSOR_SETTINGS_SUBGROUP), 
                Some(setting as *const GUID), 
                &mut val as *mut u32,
            ))
        };
        
        if res == WIN32_ERROR(0) {
            Ok(val)
        } else if res.0 == 2 {
            Err(anyhow!("Not supported"))
        } else {
            Err(anyhow!("Windows Power API error (code: {:?})", res))
        }
    }
}

pub fn display_current_settings() -> Result<()> {
    let active_guid = PowerManager::get_active_guid()?;
    let is_ac = PowerManager::get_ac_status().unwrap_or(true);
    
    let max = read_win_setting(&GUID_PROCPERFMAX, is_ac, &active_guid);
    let epp = read_win_setting(&GUID_EPP, is_ac, &active_guid);
    let boost = read_win_setting(&GUID_PERFBOOSTMODE, is_ac, &active_guid);
    let cooling = read_win_setting(&GUID_COOLING, is_ac, &active_guid);
    
    println!("--- Current Applied Settings ({}) ---", if is_ac { "AC" } else { "DC" });
    println!("Max Processor State : {}", match max { Ok(v) => format!("{}%", v), Err(e) => format!("{}", e) });
    println!("EPP                 : {}", match epp { Ok(v) => format!("{}", v), Err(e) => format!("{}", e) });
    println!("Performance Boost   : {}", match boost {
        Ok(v) => match v { 0 => "Disabled".to_string(), 1 => "Enabled".to_string(), 2 => "Aggressive".to_string(), _ => format!("{}", v) },
        Err(e) => format!("{}", e)
    });
    println!("Cooling Mode        : {}", match cooling { Ok(v) => if v == 0 { "Passive".to_string() } else { "Active".to_string() }, Err(e) => format!("{}", e) });
    println!("---------------------------------------");
    
    Ok(())
}

pub fn monitor_loop() -> Result<()> {
    println!("--- Monitoring Live Power State ---");
    let mut rust_plan_guid = PowerManager::ensure_custom_plan()?;
    loop {
        let active_guid = PowerManager::get_active_guid()?;
        if active_guid != rust_plan_guid {
            if unsafe {windows::Win32::System::Power::PowerSetActiveScheme(None, Some(&rust_plan_guid))} != windows::Win32::Foundation::WIN32_ERROR(0) {
                if let Ok(new_guid) = PowerManager::ensure_custom_plan() {
                    rust_plan_guid = new_guid;
                }
            }
        }
        let _ = display_current_settings();
        thread::sleep(Duration::from_secs(1));
    }
}
