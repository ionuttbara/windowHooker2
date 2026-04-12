use windows::core::{w, PCWSTR};
use windows::Win32::System::Registry::*;

pub struct Settings {
    pub snap_enabled: bool,
    pub alttab_enabled: bool,
    pub copilot_enabled: bool,
    pub mem_cleaner_enabled: bool,
    pub language: u32, 
}

impl Settings {
    pub fn load() -> Self {
        let mut snap = 1u32; let mut alttab = 1u32; let mut copilot = 1u32; let mut mem = 1u32; let mut lang = 0u32;
        unsafe {
            let mut hkey = HKEY::default();
            if RegCreateKeyExW(HKEY_CURRENT_USER, w!("Software\\Gallery Inc\\IBBE-Hooker"), 0, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_READ, None, &mut hkey, None).is_ok() {
                let mut size = 4u32;
                let _ = RegQueryValueExW(hkey, w!("FeatureSnap"), None, None, Some(&mut snap as *mut _ as *mut u8), Some(&mut size)); size = 4u32;
                let _ = RegQueryValueExW(hkey, w!("FeatureAltTab"), None, None, Some(&mut alttab as *mut _ as *mut u8), Some(&mut size)); size = 4u32;
                let _ = RegQueryValueExW(hkey, w!("FeatureCopilot"), None, None, Some(&mut copilot as *mut _ as *mut u8), Some(&mut size)); size = 4u32;
                let _ = RegQueryValueExW(hkey, w!("FeatureMemCleaner"), None, None, Some(&mut mem as *mut _ as *mut u8), Some(&mut size)); size = 4u32;
                let _ = RegQueryValueExW(hkey, w!("Language"), None, None, Some(&mut lang as *mut _ as *mut u8), Some(&mut size));
                let _ = RegCloseKey(hkey);
            }
        }
        Self { snap_enabled: snap != 0, alttab_enabled: alttab != 0, copilot_enabled: copilot != 0, mem_cleaner_enabled: mem != 0, language: lang }
    }

    pub fn save(&self) {
        unsafe {
            let mut hkey = HKEY::default();
            if RegCreateKeyExW(HKEY_CURRENT_USER, w!("Software\\Gallery Inc\\IBBE-Hooker"), 0, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut hkey, None).is_ok() {
                let snap = if self.snap_enabled { 1u32 } else { 0u32 };
                let alttab = if self.alttab_enabled { 1u32 } else { 0u32 };
                let copilot = if self.copilot_enabled { 1u32 } else { 0u32 };
                let mem = if self.mem_cleaner_enabled { 1u32 } else { 0u32 };
                
                let _ = RegSetValueExW(hkey, w!("FeatureSnap"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&snap as *const _ as *const u8, 4)));
                let _ = RegSetValueExW(hkey, w!("FeatureAltTab"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&alttab as *const _ as *const u8, 4)));
                let _ = RegSetValueExW(hkey, w!("FeatureCopilot"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&copilot as *const _ as *const u8, 4)));
                let _ = RegSetValueExW(hkey, w!("FeatureMemCleaner"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&mem as *const _ as *const u8, 4)));
                let _ = RegSetValueExW(hkey, w!("Language"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&self.language as *const _ as *const u8, 4)));
                let _ = RegCloseKey(hkey);
            }
        }
    }
}