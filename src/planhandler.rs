use std::ptr;
use windows::Win32::System::Power::{ACCESS_SCHEME, PowerDuplicateScheme, PowerEnumerate, PowerReadFriendlyName, PowerWriteFriendlyName};
use windows::Win32::Foundation::{HLOCAL, LocalFree, NO_ERROR, WIN32_ERROR};
use windows::core::GUID;

const GUID_BALANCED: GUID = GUID::from_u128(0x381b4222_f694_41f0_9685_ff5bb260df2e);

pub unsafe fn find_plan_by_name(target_name: &str) -> Result<GUID, String> {
    let mut index = 0;
    loop {
        let mut guid = GUID::default();
        let mut size = std::mem::size_of::<GUID>() as u32;
        
        let res = unsafe { PowerEnumerate(None, None, None, ACCESS_SCHEME, index, Some(&mut guid as *mut _ as *mut u8), &mut size) };
        if res != WIN32_ERROR(0) {
            return Err(format!("[INFO] Plan '{}' not found in system list.", target_name));
        }; 

        let mut name_buf = [0u16; 256];
        let mut name_size = (name_buf.len() * 2) as u32;

        let name_res = unsafe { PowerReadFriendlyName(None, Some(&guid), None, None, Some(name_buf.as_mut_ptr() as *mut u8), &mut name_size) };
        
        if name_res == WIN32_ERROR(0) && name_size > 0 {
            let current_name = String::from_utf16_lossy(&name_buf[..(name_size as usize / 2)])
                .trim_matches(char::from(0))
                .to_string();

            if current_name.to_lowercase() == target_name.to_lowercase() {
                println!("Found targeted PowerPlan GUID {:?}", guid);
                return Ok(guid);
            }
        }
        index += 1;
    }
}

pub unsafe fn create_rust_plan(target_name: &str) -> bool {
    let mut new_ptr: *mut GUID = ptr::null_mut();

    // 1. Duplicate
    let res = unsafe { PowerDuplicateScheme(None, &GUID_BALANCED, &mut new_ptr) };
    if res != NO_ERROR || new_ptr.is_null() {
        println!("[DEBUG] PowerDuplicateScheme failed with Win32 Error: {:?}", res);
        return false;
    }

    let new_guid = unsafe { *new_ptr }; 
    let name_u16: Vec<u16> = format!("{}\0", target_name).encode_utf16().collect();

    // 2. Rename
    let name_res = unsafe { PowerWriteFriendlyName(
        None,
        &new_guid, 
        None, 
        None, 
        std::slice::from_raw_parts(name_u16.as_ptr() as *const u8, name_u16.len() * 2),
    ) };

    if name_res != NO_ERROR {
        println!("[DEBUG] PowerWriteFriendlyName failed with Win32 Error: {:?}", name_res);
    }

    // 3. Free
    let _ = unsafe { LocalFree(Some(HLOCAL(new_ptr as *mut _))) };

    name_res == NO_ERROR
}


pub unsafe fn power_plan(target_name: &str) -> Result<GUID, String> {
    match unsafe { find_plan_by_name(target_name) } {
        Ok(guid) => Ok(guid),
        Err(_) => {
            if unsafe { create_rust_plan(target_name) } {
                unsafe { find_plan_by_name(target_name) }
            } else {
                Err(format!("CRITICAL: Could not find or create plan '{}'.", target_name))
            }
        }
    }
}
