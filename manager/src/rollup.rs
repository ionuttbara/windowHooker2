use std::collections::HashMap;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct RollUpManager {
    rolled_windows: HashMap<isize, RECT>,
}

impl RollUpManager {
    pub fn new() -> Self { Self { rolled_windows: HashMap::new() } }

    pub unsafe fn toggle(&mut self, hwnd: HWND) {
        let hwnd_val = hwnd.0 as isize;
        if let Some(original_rect) = self.rolled_windows.remove(&hwnd_val) {
            let _ = SetWindowPos(
                hwnd, HWND(0 as _), 0, 0,
                original_rect.right - original_rect.left, original_rect.bottom - original_rect.top,
                SWP_NOMOVE | SWP_NOZORDER | SWP_ASYNCWINDOWPOS,
            );
        } else {
            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            self.rolled_windows.insert(hwnd_val, rect);

            let caption = GetSystemMetrics(SM_CYCAPTION);
            let frame = GetSystemMetrics(SM_CYSIZEFRAME);
            let _ = SetWindowPos(
                hwnd, HWND(0 as _), 0, 0, 
                rect.right - rect.left, caption + frame * 2,
                SWP_NOMOVE | SWP_NOZORDER | SWP_ASYNCWINDOWPOS,
            );
        }
    }
}