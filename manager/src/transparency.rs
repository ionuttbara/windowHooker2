use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct TransparencyManager;

impl TransparencyManager {
    pub unsafe fn toggle(hwnd: HWND) {
        let style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if (style & WS_EX_LAYERED.0) != 0 {
            SetWindowLongW(hwnd, GWL_EXSTYLE, (style & !WS_EX_LAYERED.0) as i32);
        } else {
            SetWindowLongW(hwnd, GWL_EXSTYLE, (style | WS_EX_LAYERED.0) as i32);
            // 200 din 255 înseamnă ~80% opacitate
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 200, LWA_ALPHA);
        }
    }
}