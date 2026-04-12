use std::collections::HashMap;
use windows::core::w;
use windows::Win32::Foundation::{BOOL, COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

const BORDER_THICKNESS: i32 = 4;

pub struct AlwaysOnTopManager {
    overlays: HashMap<isize, HWND>,
}

impl AlwaysOnTopManager {
    pub fn new() -> Self { Self { overlays: HashMap::new() } }

    pub unsafe fn toggle(&mut self, hwnd: HWND) {
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        let is_on_top = (ex_style & WS_EX_TOPMOST.0) != 0;

        if is_on_top {
            // REZOLVARE BORDER ASCUNS: Setam fereastra Sincron
            let _ = SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            if let Some(overlay) = self.overlays.remove(&(hwnd.0 as isize)) {
                let _ = DestroyWindow(overlay);
            }
        } else {
            // REZOLVARE BORDER ASCUNS: Setam fereastra Sincron
            let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            
            let hinstance = GetModuleHandleW(None).unwrap();
            let overlay = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("WindowEnhancerOverlayClass"), w!("Overlay"), WS_POPUP,
                0, 0, 0, 0, HWND(0 as _), None, hinstance, None
            );

            if overlay.0 as usize != 0 {
                let _ = SetLayeredWindowAttributes(overlay, COLORREF(0x00FF00FF), 0, LWA_COLORKEY);
                // Am sters SetWindowDisplayAffinity care cauza conflictul de invizibilitate pe desktop composition
                
                self.overlays.insert(hwnd.0 as isize, overlay);
                self.sync_overlay(hwnd);
            }
        }
    }

    pub unsafe fn sync_overlay(&mut self, target: HWND) {
        let target_val = target.0 as isize;
        if let Some(&overlay) = self.overlays.get(&target_val) {
            if IsWindow(target).0 == 0 || (GetWindowLongW(target, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0) == 0 {
                self.overlays.remove(&target_val);
                let _ = DestroyWindow(overlay);
                return;
            }

            if IsIconic(target).0 != 0 {
                let _ = ShowWindowAsync(overlay, SW_HIDE);
                return;
            }

            let mut rect = RECT::default();
            let _ = GetWindowRect(target, &mut rect);
            
            // Fortam afisarea si actualizarea dimensiunilor
            let _ = SetWindowPos(
                overlay, HWND_TOPMOST,
                rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW
            );
            
            let _ = InvalidateRect(overlay, None, BOOL(1));
        }
    }
}

unsafe fn get_system_accent_color() -> COLORREF {
    let mut color: u32 = 0;
    let mut opaque: BOOL = BOOL(0);
    if DwmGetColorizationColor(&mut color, &mut opaque).is_ok() {
        let r = (color >> 16) & 0xFF; let g = (color >> 8) & 0xFF; let b = color & 0xFF;
        COLORREF(r | (g << 8) | (b << 16))
    } else { COLORREF(0x00FFD700) }
}

pub unsafe extern "system" fn overlay_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);

            let magenta = COLORREF(0x00FF00FF); let accent_color = get_system_accent_color();
            let brush_bg = CreateSolidBrush(magenta); FillRect(hdc, &rect, brush_bg); DeleteObject(brush_bg);
            
            let brush_border = CreateSolidBrush(accent_color);
            let t = BORDER_THICKNESS;

            let mut top = rect; top.bottom = top.top + t; 
            let mut bottom = rect; bottom.top = bottom.bottom - t;
            let mut left = rect; left.right = left.left + t; 
            let mut right = rect; right.left = right.right - t;

            FillRect(hdc, &top, brush_border); FillRect(hdc, &bottom, brush_border);
            FillRect(hdc, &left, brush_border); FillRect(hdc, &right, brush_border);

            DeleteObject(brush_border); EndPaint(hwnd, &ps); LRESULT(0)
        }
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as _),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}