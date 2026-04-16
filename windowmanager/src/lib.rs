pub mod alttabov;
pub mod winnsnap;
pub mod fancyzone; // Registered the new module

pub static mut SNAP_ENABLED: bool = true;
pub static mut ALTTAB_ENABLED: bool = true;

pub struct DesktopManager;

impl DesktopManager {
    pub unsafe fn initialize_all() {
        winnsnap::WindowSnapper::init();
        alttabov::AltTabManager::init();
        fancyzone::FancyZoneManager::new(); // Initialized the manager
    }
    
    pub unsafe fn cleanup_all() {
        winnsnap::WindowSnapper::cleanup();
        alttabov::AltTabManager::cleanup();
        
        // La închidere, repornim snap-ul original din Windows
        use windows::Win32::UI::WindowsAndMessaging::*;
        let _ = SystemParametersInfoW(SPI_SETDOCKMOVING, 1, None, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
        let _ = SystemParametersInfoW(SPI_SETSNAPSIZING, 1, None, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
    }
    
    pub unsafe fn restore_if_snapped(hwnd: windows::Win32::Foundation::HWND) {
        winnsnap::WindowSnapper::restore_if_snapped(hwnd);
    }
    
    pub unsafe fn set_snap_enabled(enabled: bool) { 
        SNAP_ENABLED = enabled; 
        
        use windows::Win32::UI::WindowsAndMessaging::*;
        // Dacă e activat managerul nostru, dezactivăm funcția nativă (0). Altfel, o activăm (1).
        let val = if enabled { 0 } else { 1 };
        let _ = SystemParametersInfoW(SPI_SETDOCKMOVING, val, None, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
        let _ = SystemParametersInfoW(SPI_SETSNAPSIZING, val, None, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
    }
    
    pub unsafe fn set_alttab_enabled(enabled: bool) { ALTTAB_ENABLED = enabled; }
}