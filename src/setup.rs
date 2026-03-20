use windows::Win32::Foundation::{HANDLE, CloseHandle};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use std::io::Write;
use std::{env, io};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::Context;

const INSTALL_DIR: &str = r"C:\Program Files\RustGovernor";
const APP_NAME: &str = "rust-governor.exe";

fn wait_for_exit() {
    println!("\nPress Enter to exit...");
    let _ = io::stdin().read_line(&mut String::new());
}

fn confirm_action(message: &str) -> bool {
    print!("{} (y/N): ", message);
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let input = input.trim().to_lowercase();
        return input == "y" || input == "yes";
    }
    false
}

fn is_admin() -> bool {
    unsafe {
        let mut token: HANDLE = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_ok() {
            let mut elevation = TOKEN_ELEVATION::default();
            let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
            
            if GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                size,
                &mut size,
            ).is_ok() {
                let _ = CloseHandle(token); 
                return elevation.TokenIsElevated != 0;
            }
            let _ = CloseHandle(token);
        }
        false
    }
}

pub fn self_install() -> anyhow::Result<()> {
    if !confirm_action(r"Do you want to install RustGovernor to C:\Program Files\RustGovernor") {
        println!("Installation cancelled.");
        return Ok(())
    }
    if !is_admin() {
        anyhow::bail!("Error: Administrator privileges required to install RustGovernor.");
    }
    let current_pid = std::process::id().to_string();
    let _ = Command::new("powershell")
        .args([
        "-Command",
        &format!("Get-Process rust-governor -ErrorAction SilentlyContinue | Where-Object {{ $_.Id -ne {} }} | Stop-Process -Force", current_pid)        
        ])
        .status();
   
    println!("Stopped background instances.");
    let exe_path = env::current_exe()?;
    let install_path = PathBuf::from(INSTALL_DIR).join(APP_NAME);

    if !Path::new(INSTALL_DIR).exists() {
        fs::create_dir_all(INSTALL_DIR)?;
    }

    if exe_path != install_path {
        fs::copy(&exe_path, &install_path)?;
        println!("Binary copied to {}", INSTALL_DIR);
    } else {
        println!("Running from installation directory.");
    }
    
    let status = Command::new("schtasks")
    .args([
        "/Create",
        "/TN", "RustGovernor",
        "/TR", &format!("cmd /c start /b \"\" \"{}\" --run", install_path.to_str().unwrap()),
        "/SC", "ONLOGON",
        "/RL", "HIGHEST",
        "/F"
    ])
    .status()?;
    if status.success() {
    println!("Registered in System Startup (All Users).");
    }
    let _ = create_global_shortcut();
    println!("Installation Complete!");
    wait_for_exit();
    Ok(())
}

pub fn uninstall() -> anyhow::Result<()> {
    if !confirm_action("Are you sure you want to Remove RustGovernor?") {
        println!("Uninstallation cancelled.");
        return Ok(());
    }
    if !is_admin() {
        anyhow::bail!("Error: Administrator privileges required to uninstall RustGovernor.");
    }

    let _ = Command::new("schtasks")
         .args(["/Delete", "/TN", "RustGovernor", "/F"])
         .status();

    let current_pid = std::process::id().to_string();
    let _ = Command::new("powershell")
        .args([
        "-Command",
        &format!("Get-Process rust-governor -ErrorAction SilentlyContinue | Where-Object {{ $_.Id -ne {} }} | Stop-Process -Force", current_pid)        
        ])
        .status();
   
    println!("Stopped background instances.");

    let exe_path = std::env::current_exe()?;
        if let Some(exe_dir) = exe_path.parent() {
            let _ = remove_global_shortcut();
            println!("Cleaned up shortcuts. You can now remove the folder here {}", exe_dir.display());
        }
    println!("Uninstallation complete!");
    wait_for_exit();
    Ok(())
}

pub fn create_global_shortcut() -> anyhow::Result<()> {
    let batch_path = r"C:\Windows\rust-governor.bat";
    let content = r#"@"C:\Program Files\RustGovernor\rust-governor.exe" %*"#;
    
    fs::write(batch_path, content)
        .context("Failed to create global command. Ensure you are running with Administrator privileges.")?;
    
    println!("Success: 'rust-governor' is now accessible from any terminal.");
    Ok(())
}

pub fn remove_global_shortcut() -> anyhow::Result<()> {
    let batch_path = r"C:\Windows\rust-governor.bat";
    if std::path::Path::new(batch_path).exists() {
        fs::remove_file(batch_path).context("Failed to remove the global shortcut.")?;
    }
    Ok(())
}