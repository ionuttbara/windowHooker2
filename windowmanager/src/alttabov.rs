use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::mem::size_of;
use std::ptr::{addr_of, addr_of_mut}; 
use windows::core::w;
use windows::Win32::Foundation::{BOOL, COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId}; 
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use crate::winnsnap::{WindowSnapper, SnapDir};

const WM_CUSTOM_TAB_PRESSED: u32 = WM_USER + 101;
const WM_CUSTOM_ALT_RELEASED: u32 = WM_USER + 102;
const WM_CUSTOM_MOUSE_SCROLL: u32 = WM_USER + 103;

static mut KBD_HOOK: HHOOK = HHOOK(0 as _);
static mut MOUSE_HOOK: HHOOK = HHOOK(0 as _); 
static mut ALTTAB_HWND: HWND = HWND(0 as _);
static mut UI_FONT: HFONT = HFONT(0 as _);    

static mut WINDOW_LIST: Vec<(HWND, String, HICON)> = Vec::new();
static mut SELECTED_INDEX: usize = 0;
static mut IS_ALTTABBING: bool = false;
static mut THUMBNAIL_HANDLE: isize = 0;       

pub struct AltTabManager;

impl AltTabManager {
    pub unsafe fn init() {
        let h_inst = GetModuleHandleW(None).unwrap();
        
        let wc = WNDCLASSW {
            lpfnWndProc: Some(alttab_proc),
            hInstance: h_inst.into(),
            lpszClassName: w!("CustomAltTabClass"),
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);

        ALTTAB_HWND = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST, 
            w!("CustomAltTabClass"), w!("AltTabUI"), WS_POPUP,
            0, 0, 0, 0, HWND(0 as _), None, h_inst, None
        );

        UI_FONT = CreateFontW(
            22, 0, 0, 0, 600, 0, 0, 0, DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32, CLIP_DEFAULT_PRECIS.0 as u32,
            CLEARTYPE_QUALITY.0 as u32, DEFAULT_PITCH.0 as u32, w!("Segoe UI")
        );

        KBD_HOOK = SetWindowsHookExW(WH_KEYBOARD_LL, Some(kbd_proc), h_inst, 0).unwrap();
    }

    pub unsafe fn cleanup() {
        if KBD_HOOK.0 as usize != 0 { let _ = UnhookWindowsHookEx(KBD_HOOK); }
        if MOUSE_HOOK.0 as usize != 0 { let _ = UnhookWindowsHookEx(MOUSE_HOOK); }
        if ALTTAB_HWND.0 as usize != 0 { let _ = DestroyWindow(ALTTAB_HWND); }
        if UI_FONT.0 as usize != 0 { let _ = DeleteObject(UI_FONT); }
    }
}

unsafe fn force_foreground(hwnd: HWND) {
    let fg_hwnd = GetForegroundWindow();
    let fg_thread = GetWindowThreadProcessId(fg_hwnd, None);
    let my_thread = GetCurrentThreadId();

    if IsIconic(hwnd).as_bool() {
        let _ = ShowWindow(hwnd, SW_RESTORE);
    } else {
        let _ = ShowWindow(hwnd, SW_SHOW);
    }

    if fg_thread != my_thread && fg_hwnd.0 as usize != 0 {
        let _ = AttachThreadInput(fg_thread, my_thread, BOOL(1));
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        let _ = AttachThreadInput(fg_thread, my_thread, BOOL(0));
    } else {
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
    }
}

unsafe fn update_thumbnail() {
    if THUMBNAIL_HANDLE != 0 {
        let _ = DwmUnregisterThumbnail(THUMBNAIL_HANDLE);
        THUMBNAIL_HANDLE = 0;
    }
    
    // Convertim pointerul brut la o referinta explicita
    if (&*addr_of!(WINDOW_LIST)).is_empty() { return; }
    
    let target = (&*addr_of!(WINDOW_LIST))[SELECTED_INDEX].0;
    
    if let Ok(thumb_handle) = DwmRegisterThumbnail(ALTTAB_HWND, target) {
        THUMBNAIL_HANDLE = thumb_handle;
        
        let screen_w = GetSystemMetrics(SM_CXSCREEN) as f32;
        let screen_h = GetSystemMetrics(SM_CYSCREEN) as f32;
        
        let mut placement = WINDOWPLACEMENT { length: size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
        let _ = GetWindowPlacement(target, &mut placement);
        
        let mut win_rect = placement.rcNormalPosition;
        if placement.showCmd == SW_SHOWMAXIMIZED.0 as u32 {
            win_rect.left = 0; win_rect.top = 0;
            win_rect.right = screen_w as i32; win_rect.bottom = screen_h as i32;
        } else if placement.showCmd != SW_SHOWMINIMIZED.0 as u32 {
            let mut frame_rect = RECT::default();
            if DwmGetWindowAttribute(target, DWMWA_EXTENDED_FRAME_BOUNDS, &mut frame_rect as *mut _ as _, size_of::<RECT>() as u32).is_ok() {
                win_rect = frame_rect;
            } else {
                let _ = GetWindowRect(target, &mut win_rect);
            }
        }

        let preview_x = 350.0; let preview_y = 30.0;
        let preview_w = 540.0; let preview_h = 460.0;

        let scale = (preview_w / screen_w).min(preview_h / screen_h);
        let map_x = preview_x + (preview_w - screen_w * scale) / 2.0;
        let map_y = preview_y + (preview_h - screen_h * scale) / 2.0;

        let thumb_left = (map_x + win_rect.left as f32 * scale) as i32;
        let thumb_top = (map_y + win_rect.top as f32 * scale) as i32;
        let thumb_right = (map_x + win_rect.right as f32 * scale) as i32;
        let thumb_bottom = (map_y + win_rect.bottom as f32 * scale) as i32;

        let mut props = DWM_THUMBNAIL_PROPERTIES::default();
        props.dwFlags = DWM_TNP_VISIBLE | DWM_TNP_RECTDESTINATION | DWM_TNP_OPACITY;
        props.fVisible = BOOL(1);
        props.opacity = 255;
        props.rcDestination = RECT { left: thumb_left, top: thumb_top, right: thumb_right, bottom: thumb_bottom };
        
        let _ = DwmUpdateThumbnailProperties(thumb_handle, &props);
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && wparam.0 as u32 == WM_MOUSEWHEEL {
        let ms = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        let delta = ((ms.mouseData >> 16) & 0xFFFF) as i16;
        let is_up = if delta > 0 { 1 } else { 0 };
        let _ = PostMessageW(ALTTAB_HWND, WM_CUSTOM_MOUSE_SCROLL, WPARAM(is_up), LPARAM(0));
        return LRESULT(1);
    }
    CallNextHookEx(MOUSE_HOOK, code, wparam, lparam)
}

unsafe extern "system" fn kbd_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let kbd = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let event = wparam.0 as u32;
        let is_key_down = event == WM_KEYDOWN || event == WM_SYSKEYDOWN;
        let is_key_up = event == WM_KEYUP || event == WM_SYSKEYUP;

        let is_win_down = GetAsyncKeyState(VK_LWIN.0 as i32) < 0 || GetAsyncKeyState(VK_RWIN.0 as i32) < 0;
        if is_win_down && is_key_down {
            let fw = GetForegroundWindow();
            match VIRTUAL_KEY(kbd.vkCode as u16) {
                VK_LEFT => { WindowSnapper::snap_active(fw, SnapDir::Left); return LRESULT(1); }
                VK_RIGHT => { WindowSnapper::snap_active(fw, SnapDir::Right); return LRESULT(1); }
                VK_UP => { WindowSnapper::snap_active(fw, SnapDir::Up); return LRESULT(1); }
                VK_DOWN => { WindowSnapper::snap_active(fw, SnapDir::Down); return LRESULT(1); }
                _ => {}
            }
        }

        let is_alt_down = (kbd.flags.0 & LLKHF_ALTDOWN.0) != 0;

        if (kbd.vkCode == VK_LMENU.0 as u32 || kbd.vkCode == VK_RMENU.0 as u32) && is_key_up {
            let _ = PostMessageW(ALTTAB_HWND, WM_CUSTOM_ALT_RELEASED, WPARAM(0), LPARAM(0));
        }

        if kbd.vkCode == VK_TAB.0 as u32 && is_alt_down {
            if is_key_down {
                let is_shift_down = GetAsyncKeyState(VK_LSHIFT.0 as i32) < 0 || GetAsyncKeyState(VK_RSHIFT.0 as i32) < 0;
                let shift_param = if is_shift_down { 1 } else { 0 };
                let _ = PostMessageW(ALTTAB_HWND, WM_CUSTOM_TAB_PRESSED, WPARAM(shift_param), LPARAM(0));
            }
            return LRESULT(1); 
        }
    }
    CallNextHookEx(KBD_HOOK, code, wparam, lparam)
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, _: LPARAM) -> BOOL {
    if IsWindowVisible(hwnd).as_bool() {
        let mut title = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut title);
        if len > 0 {
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            if (ex_style & WS_EX_TOOLWINDOW.0) == 0 {
                let mut cloaked = 0u32;
                if DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as _, size_of::<u32>() as u32).is_ok() && cloaked != 0 {
                    return BOOL(1);
                }

                let title_str = OsString::from_wide(&title[..len as usize]).to_string_lossy().to_string();
                if title_str != "Manager" && title_str != "SnapOverlay" && title_str != "AltTabUI" && title_str != "Program Manager" {
                    
                    let mut hicon_isize = SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_SMALL as _), LPARAM(0)).0 as isize;
                    if hicon_isize == 0 {
                        #[cfg(target_arch = "x86_64")] { hicon_isize = GetClassLongPtrW(hwnd, GCLP_HICONSM) as isize; }
                        #[cfg(target_arch = "x86")] { hicon_isize = GetClassLongW(hwnd, GCLP_HICONSM) as isize; }
                    }
                    let hicon = if hicon_isize != 0 { HICON(hicon_isize) } else { LoadIconW(None, IDI_APPLICATION).unwrap() };

                    (&mut *addr_of_mut!(WINDOW_LIST)).push((hwnd, title_str, hicon));
                }
            }
        }
    }
    BOOL(1)
}

unsafe extern "system" fn alttab_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CUSTOM_TAB_PRESSED => {
            let is_shift_down = wparam.0 == 1;
            if !IS_ALTTABBING {
                (&mut *addr_of_mut!(WINDOW_LIST)).clear();
                let _ = EnumWindows(Some(enum_windows_proc), LPARAM(0));
                
                if (&*addr_of!(WINDOW_LIST)).is_empty() { return LRESULT(0); }
                
                IS_ALTTABBING = true;
                SELECTED_INDEX = if (&*addr_of!(WINDOW_LIST)).len() > 1 { 1 } else { 0 };

                let h_inst = GetModuleHandleW(None).unwrap();
                MOUSE_HOOK = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), h_inst, 0).unwrap();

                let screen_w = GetSystemMetrics(SM_CXSCREEN); 
                let screen_h = GetSystemMetrics(SM_CYSCREEN);
                let w = 920; let h = 520;
                let _ = SetWindowPos(ALTTAB_HWND, HWND_TOPMOST, (screen_w - w) / 2, (screen_h - h) / 2, w, h, SWP_SHOWWINDOW);
                
                update_thumbnail();
            } else {
                if is_shift_down {
                    SELECTED_INDEX = if SELECTED_INDEX == 0 { (&*addr_of!(WINDOW_LIST)).len() - 1 } else { SELECTED_INDEX - 1 };
                } else {
                    SELECTED_INDEX = (SELECTED_INDEX + 1) % (&*addr_of!(WINDOW_LIST)).len();
                }
                update_thumbnail();
            }
            let _ = InvalidateRect(ALTTAB_HWND, None, BOOL(1));
            let _ = UpdateWindow(ALTTAB_HWND);
            LRESULT(0)
        }
        WM_CUSTOM_MOUSE_SCROLL => {
            let is_up = wparam.0 == 1;
            if is_up { 
                SELECTED_INDEX = if SELECTED_INDEX == 0 { (&*addr_of!(WINDOW_LIST)).len().saturating_sub(1) } else { SELECTED_INDEX - 1 };
            } else { 
                SELECTED_INDEX = (SELECTED_INDEX + 1) % (&*addr_of!(WINDOW_LIST)).len().max(1);
            }
            update_thumbnail();
            let _ = InvalidateRect(ALTTAB_HWND, None, BOOL(1));
            let _ = UpdateWindow(ALTTAB_HWND);
            LRESULT(0)
        }
        WM_CUSTOM_ALT_RELEASED => {
            if IS_ALTTABBING {
                IS_ALTTABBING = false;
                
                if THUMBNAIL_HANDLE != 0 {
                    let _ = DwmUnregisterThumbnail(THUMBNAIL_HANDLE);
                    THUMBNAIL_HANDLE = 0;
                }
                if MOUSE_HOOK.0 as usize != 0 {
                    let _ = UnhookWindowsHookEx(MOUSE_HOOK);
                    MOUSE_HOOK = HHOOK(0 as _);
                }

                let _ = ShowWindowAsync(ALTTAB_HWND, SW_HIDE);

                if (&*addr_of!(WINDOW_LIST)).len() > SELECTED_INDEX {
                    let target = (&*addr_of!(WINDOW_LIST))[SELECTED_INDEX].0;
                    force_foreground(target);
                }
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rect = RECT::default(); let _ = GetClientRect(hwnd, &mut rect);
            
            let brush_bg = CreateSolidBrush(COLORREF(0x001A1A1A)); 
            FillRect(hdc, &rect, brush_bg); DeleteObject(brush_bg);
            
            let brush_border = CreateSolidBrush(COLORREF(0x00404040));
            FrameRect(hdc, &rect, brush_border); DeleteObject(brush_border);

            let panel_rect = RECT { left: 1, top: 1, right: 330, bottom: rect.bottom - 1 };
            let brush_panel = CreateSolidBrush(COLORREF(0x000F0F0F));
            FillRect(hdc, &panel_rect, brush_panel); DeleteObject(brush_panel);

            let mut color: u32 = 0; let mut opaque: BOOL = BOOL(0);
            let accent_color = if DwmGetColorizationColor(&mut color, &mut opaque).is_ok() {
                let r = (color >> 16) & 0xFF; let g = (color >> 8) & 0xFF; let b = color & 0xFF;
                COLORREF(r | (g << 8) | (b << 16))
            } else { COLORREF(0x00D050D0) }; 

            let r_val = (accent_color.0 & 0xFF) as u32;
            let g_val = ((accent_color.0 >> 8) & 0xFF) as u32;
            let b_val = ((accent_color.0 >> 16) & 0xFF) as u32;
            let luminance = (299 * r_val + 587 * g_val + 114 * b_val) / 1000;
            let active_text_color = if luminance > 128 { COLORREF(0x00000000) } else { COLORREF(0x00FFFFFF) };

            let screen_w = GetSystemMetrics(SM_CXSCREEN) as f32;
            let screen_h = GetSystemMetrics(SM_CYSCREEN) as f32;
            let scale = (540.0 / screen_w).min(460.0 / screen_h);
            
            let sx = (350.0 + (540.0 - screen_w * scale) / 2.0) as i32;
            let sy = (30.0 + (460.0 - screen_h * scale) / 2.0) as i32;
            let sw = (screen_w * scale) as i32;
            let sh = (screen_h * scale) as i32;

            let monitor_rect = RECT { left: sx, top: sy, right: sx + sw, bottom: sy + sh };
            let brush_monitor = CreateSolidBrush(COLORREF(0x00080808)); 
            FillRect(hdc, &monitor_rect, brush_monitor); DeleteObject(brush_monitor);
            let brush_monitor_border = CreateSolidBrush(COLORREF(0x00333333));
            FrameRect(hdc, &monitor_rect, brush_monitor_border); DeleteObject(brush_monitor_border);

            let old_font = SelectObject(hdc, UI_FONT);
            let _ = SetBkMode(hdc, TRANSPARENT as _);
            
            let max_visible = 10;
            let start_idx = if SELECTED_INDEX >= max_visible { SELECTED_INDEX - max_visible + 1 } else { 0 };

            for i in 0..max_visible {
                let act_idx = start_idx + i;
                if act_idx >= (&*addr_of!(WINDOW_LIST)).len() { break; }
                let (_, title, hicon) = &(&*addr_of!(WINDOW_LIST))[act_idx];

                let y_pos = 30 + (i as i32 * 45); 
                
                if act_idx == SELECTED_INDEX {
                    let hl_rect = RECT { left: 10, top: y_pos - 5, right: 320, bottom: y_pos + 35 };
                    let brush_hl = CreateSolidBrush(accent_color); 
                    FillRect(hdc, &hl_rect, brush_hl); DeleteObject(brush_hl);
                    let _ = SetTextColor(hdc, active_text_color); 
                } else {
                    let _ = SetTextColor(hdc, COLORREF(0x00888888)); 
                }

                let _ = DrawIconEx(hdc, 20, y_pos, *hicon, 24, 24, 0, None, DI_NORMAL);

                let w_title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
                let mut text_rect = RECT { left: 55, top: y_pos + 4, right: 310, bottom: y_pos + 30 };
                let _ = DrawTextW(hdc, &mut w_title.clone(), &mut text_rect, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
            }

            SelectObject(hdc, old_font);
            EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}