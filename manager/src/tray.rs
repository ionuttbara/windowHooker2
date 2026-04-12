use std::collections::HashMap;
use shared::WM_TRAY_CALLBACK;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct TrayManager {
    tray_icons: HashMap<u32, HWND>,
}

impl TrayManager {
    pub fn new() -> Self { Self { tray_icons: HashMap::new() } }

    pub unsafe fn send_to_tray(&mut self, hwnd: HWND, manager_hwnd: HWND) {
        let tray_id = (hwnd.0 as usize & 0xFFFFFFFF) as u32;
        if tray_id == 1 { return; } 

        let mut result: usize = 0;
        // REZOLVARE: Am invelit variabila in Some(...)
        let _ = SendMessageTimeoutW(hwnd, WM_GETICON, WPARAM(ICON_SMALL as _), LPARAM(0), SMTO_ABORTIFHUNG, 100, Some(&mut result as *mut usize));
        let mut hicon_isize = result as isize;
        
        if hicon_isize == 0 {
            #[cfg(target_arch = "x86_64")]
            { hicon_isize = GetClassLongPtrW(hwnd, GCLP_HICONSM) as isize; }
            #[cfg(target_arch = "x86")]
            { hicon_isize = GetClassLongW(hwnd, GCLP_HICONSM) as isize; }
        }
        
        let hicon = if hicon_isize != 0 { HICON(hicon_isize) } else { LoadIconW(None, IDI_APPLICATION).unwrap() };

        let mut title = [0u16; 128];
        GetWindowTextW(hwnd, &mut title);

        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = manager_hwnd; nid.uID = tray_id; nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY_CALLBACK; nid.hIcon = hicon; nid.szTip = title;

        Shell_NotifyIconW(NIM_ADD, &nid);
        self.tray_icons.insert(tray_id, hwnd);
        
        ShowWindowAsync(hwnd, SW_HIDE);
    }

    pub unsafe fn restore_from_tray(&mut self, tray_id: u32, manager_hwnd: HWND) {
        if let Some(&hwnd) = self.tray_icons.get(&tray_id) {
            ShowWindowAsync(hwnd, SW_SHOW);
            ShowWindowAsync(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
            
            let mut nid = NOTIFYICONDATAW::default();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = manager_hwnd; nid.uID = tray_id;
            Shell_NotifyIconW(NIM_DELETE, &nid);
            
            self.tray_icons.remove(&tray_id);
        }
    }

    pub unsafe fn close_from_tray(&mut self, tray_id: u32, manager_hwnd: HWND) {
        if let Some(&hwnd) = self.tray_icons.get(&tray_id) {
            let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
            
            let mut nid = NOTIFYICONDATAW::default();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = manager_hwnd; nid.uID = tray_id;
            Shell_NotifyIconW(NIM_DELETE, &nid);
            
            self.tray_icons.remove(&tray_id);
        }
    }
}