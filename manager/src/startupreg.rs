use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::w;
use windows::Win32::System::Registry::*;

pub struct StartupManager;

impl StartupManager {
    const APP_KEY: windows::core::PCWSTR = w!("IBB-Hooker");

    pub unsafe fn is_registered() -> bool {
        let mut hkey = HKEY::default();
        // VERIFICARE NOUĂ: folosim .is_ok() in loc de == ERROR_SUCCESS
        if RegOpenKeyExW(HKEY_CURRENT_USER, w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"), 0, KEY_READ, &mut hkey).is_ok() {
            let mut data_type = REG_VALUE_TYPE(0); // Setam explicit tipul cerut
            let status = RegQueryValueExW(hkey, Self::APP_KEY, None, Some(&mut data_type), None, None);
            let _ = RegCloseKey(hkey);
            return status.is_ok();
        }
        false
    }

    pub unsafe fn toggle() -> bool {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"), 0, KEY_SET_VALUE | KEY_READ, &mut hkey).is_ok() {
            if Self::is_registered() {
                let _ = RegDeleteValueW(hkey, Self::APP_KEY);
                let _ = RegCloseKey(hkey);
                return false; 
            } else {
                if let Ok(exe_path) = std::env::current_exe() {
                    let mut path_str: Vec<u16> = OsStr::new(exe_path.as_os_str()).encode_wide().collect();
                    path_str.push(0); 
                    
                    let path_bytes = std::slice::from_raw_parts(path_str.as_ptr() as *const u8, path_str.len() * 2);
                    let _ = RegSetValueExW(hkey, Self::APP_KEY, 0, REG_SZ, Some(path_bytes));
                }
                let _ = RegCloseKey(hkey);
                return true; 
            }
        }
        false
    }
}