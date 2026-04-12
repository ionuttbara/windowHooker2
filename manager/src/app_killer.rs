use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, MAX_PATH};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
    PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

pub struct AppKiller;

impl AppKiller {
    pub unsafe fn kill_target(hwnd: HWND) {
        if hwnd.0 as usize == 0 { return; }
        
        let mut pid = 0;
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        
        if pid != 0 {
            // Deschidem procesul cu drepturi de citire a numelui și de terminare
            if let Ok(h_process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE, false, pid) {
                
                let mut buffer = [0u16; MAX_PATH as usize];
                let mut size = MAX_PATH;
                
                // Extragem numele executabilului
                if QueryFullProcessImageNameW(h_process, PROCESS_NAME_FORMAT(0), PWSTR(buffer.as_mut_ptr()), &mut size).is_ok() {
                    let process_name = OsString::from_wide(&buffer[..size as usize]);
                    let process_name_str = process_name.to_string_lossy().to_lowercase();
                    
                    // EXCEPȚIE: Prevenim închiderea procesului Windows Explorer (Taskbar / Foldere)
                    if process_name_str.ends_with("explorer.exe") {
                        let _ = CloseHandle(h_process);
                        return;
                    }
                }
                
                // Dacă nu e explorer, îl distrugem
                let _ = TerminateProcess(h_process, 1);
                let _ = CloseHandle(h_process);
            }
        }
    }
}