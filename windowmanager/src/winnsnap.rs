use std::mem::size_of;
use std::ptr::{addr_of, addr_of_mut}; 
use windows::core::w;
use windows::Win32::Foundation::{BOOL, COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM, POINT};
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{SetCapture, ReleaseCapture};

static mut SNAP_HOOK_START: HWINEVENTHOOK = HWINEVENTHOOK(0 as _);
static mut SNAP_HOOK_MOVE: HWINEVENTHOOK = HWINEVENTHOOK(0 as _);
static mut SNAP_HOOK_END: HWINEVENTHOOK = HWINEVENTHOOK(0 as _);

static mut DRAGGED_HWND: HWND = HWND(0 as _);
static mut OVERLAY_HWND: HWND = HWND(0 as _);
static mut PENDING_SNAP_RECT: Option<RECT> = None;

static mut PRE_SNAP_RECTS: Vec<(HWND, RECT)> = Vec::new();
static mut SNAPPED_WINDOWS: Vec<HWND> = Vec::new();

// --- VARIABILE PENTRU SISTEMUL DE REDIMENSIONARE (SMART RESIZE) ---
static mut RESIZE_LINE_HWND: HWND = HWND(0 as _);
static mut CURRENT_HOVER_EDGE: Option<EdgeDef> = None;
static mut IS_DRAGGING: bool = false;
static mut DRAG_START_PT: POINT = POINT { x: 0, y: 0 };
static mut DRAG_W1_RECT: RECT = RECT { left: 0, top: 0, right: 0, bottom: 0 };
static mut DRAG_W2_RECT: RECT = RECT { left: 0, top: 0, right: 0, bottom: 0 };

#[derive(Clone, Copy)]
pub enum SnapDir { Left, Right, Up, Down }

#[derive(Clone, Copy)]
struct EdgeDef {
    w1: HWND, w2: HWND,
    is_vertical: bool, 
    rect: RECT,
}

unsafe fn is_fullscreen(hwnd: HWND) -> bool {
    let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
    if (style & WS_CAPTION.0) == 0 { return true; }

    let mut rect = RECT::default();
    if GetWindowRect(hwnd, &mut rect).is_err() { return false; }
    
    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    if monitor.0 as usize == 0 { return false; }

    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    if GetMonitorInfoW(monitor, &mut mi).as_bool() {
        return rect.left <= mi.rcMonitor.left && rect.top <= mi.rcMonitor.top &&
               rect.right >= mi.rcMonitor.right && rect.bottom >= mi.rcMonitor.bottom;
    }
    false
}

unsafe fn get_true_rect(hwnd: HWND) -> RECT {
    let mut r = RECT::default();
    if DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut _ as _, size_of::<RECT>() as u32).is_ok() { r } 
    else { let _ = GetWindowRect(hwnd, &mut r); r }
}

unsafe fn get_shared_edges() -> Vec<EdgeDef> {
    let mut edges = Vec::new();
    (&mut *addr_of_mut!(SNAPPED_WINDOWS)).retain(|&w| IsWindow(w).0 != 0 && IsWindowVisible(w).0 != 0 && IsIconic(w).0 == 0);
    
    let snapped = &*addr_of!(SNAPPED_WINDOWS);
    let count = snapped.len();
    if count < 2 { return edges; }

    for i in 0..count {
        for j in (i + 1)..count {
            let w1 = snapped[i]; let w2 = snapped[j];
            let r1 = get_true_rect(w1); let r2 = get_true_rect(w2);

            let v_overlap = r1.top.max(r2.top) < r1.bottom.min(r2.bottom);
            let h_overlap = r1.left.max(r2.left) < r1.right.min(r2.right);

            if v_overlap {
                if (r1.right - r2.left).abs() <= 10 {
                    edges.push(EdgeDef { w1, w2, is_vertical: true, rect: RECT { left: r1.right, top: r1.top.max(r2.top), right: r1.right, bottom: r1.bottom.min(r2.bottom) } });
                } else if (r1.left - r2.right).abs() <= 10 {
                    edges.push(EdgeDef { w1: w2, w2: w1, is_vertical: true, rect: RECT { left: r2.right, top: r1.top.max(r2.top), right: r2.right, bottom: r1.bottom.min(r2.bottom) } });
                }
            }
            if h_overlap {
                if (r1.bottom - r2.top).abs() <= 10 {
                    edges.push(EdgeDef { w1, w2, is_vertical: false, rect: RECT { left: r1.left.max(r2.left), top: r1.bottom, right: r1.right.min(r2.right), bottom: r1.bottom } });
                } else if (r1.top - r2.bottom).abs() <= 10 {
                    edges.push(EdgeDef { w1: w2, w2: w1, is_vertical: false, rect: RECT { left: r1.left.max(r2.left), top: r2.bottom, right: r1.right.min(r2.right), bottom: r2.bottom } });
                }
            }
        }
    }
    edges
}

pub struct WindowSnapper;

impl WindowSnapper {
    pub unsafe fn init() {
        let hinstance = GetModuleHandleW(None).unwrap();
        
        let wc = WNDCLASSW { lpfnWndProc: Some(overlay_proc), hInstance: hinstance.into(), lpszClassName: w!("SnapOverlayClass"), ..Default::default() };
        let _ = RegisterClassW(&wc);

        OVERLAY_HWND = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_TRANSPARENT,
            w!("SnapOverlayClass"), w!("SnapOverlay"), WS_POPUP,
            0, 0, 0, 0, HWND(0 as _), None, hinstance, None
        );
        let _ = SetLayeredWindowAttributes(OVERLAY_HWND, COLORREF(0), 120, LWA_ALPHA);

        let wc_line = WNDCLASSW { lpfnWndProc: Some(resize_line_proc), hInstance: hinstance.into(), lpszClassName: w!("ResizeLineClass"), ..Default::default() };
        let _ = RegisterClassW(&wc_line);
        RESIZE_LINE_HWND = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            w!("ResizeLineClass"), w!("ResizeLine"), WS_POPUP,
            0, 0, 0, 0, HWND(0 as _), None, hinstance, None
        );
        let _ = SetLayeredWindowAttributes(RESIZE_LINE_HWND, COLORREF(0), 255, LWA_ALPHA);
        let _ = SetTimer(OVERLAY_HWND, 1001, 50, None); 

        SNAP_HOOK_START = SetWinEventHook(EVENT_SYSTEM_MOVESIZESTART, EVENT_SYSTEM_MOVESIZESTART, None, Some(snap_event_proc), 0, 0, WINEVENT_OUTOFCONTEXT);
        SNAP_HOOK_MOVE = SetWinEventHook(EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE, None, Some(snap_event_proc), 0, 0, WINEVENT_OUTOFCONTEXT);
        SNAP_HOOK_END = SetWinEventHook(EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZEEND, None, Some(snap_event_proc), 0, 0, WINEVENT_OUTOFCONTEXT);
    }

    pub unsafe fn cleanup() {
        if OVERLAY_HWND.0 as usize != 0 { let _ = DestroyWindow(OVERLAY_HWND); }
        if RESIZE_LINE_HWND.0 as usize != 0 { let _ = DestroyWindow(RESIZE_LINE_HWND); }
        if SNAP_HOOK_START.0 as usize != 0 { UnhookWinEvent(SNAP_HOOK_START); }
        if SNAP_HOOK_MOVE.0 as usize != 0 { UnhookWinEvent(SNAP_HOOK_MOVE); }
        if SNAP_HOOK_END.0 as usize != 0 { UnhookWinEvent(SNAP_HOOK_END); }
    }

    pub unsafe fn restore_if_snapped(hwnd: HWND) {
        if (&*addr_of!(SNAPPED_WINDOWS)).contains(&hwnd) {
            (&mut *addr_of_mut!(SNAPPED_WINDOWS)).retain(|&x| x != hwnd);
            if let Some(idx) = (&*addr_of!(PRE_SNAP_RECTS)).iter().position(|x| x.0 == hwnd) {
                let orig_rect = (&*addr_of!(PRE_SNAP_RECTS))[idx].1;
                let w = orig_rect.right - orig_rect.left; let h = orig_rect.bottom - orig_rect.top;
                let mut pt = POINT::default(); let _ = GetCursorPos(&mut pt);
                let new_x = pt.x - (w / 2); let new_y = pt.y - 15; 
                let _ = SetWindowPos(hwnd, HWND(0 as _), new_x, new_y, w, h, SWP_NOZORDER);
            }
        }
    }

    pub unsafe fn snap_active(hwnd: HWND, dir: SnapDir) {
        if hwnd.0 as usize == 0 || is_fullscreen(hwnd) { return; }
        
        let mut wa = RECT::default();
        let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut wa as *mut _ as _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
        let w = wa.right - wa.left; let h = wa.bottom - wa.top;
        let is_ultrawide = (w as f32 / h as f32) >= 2.3;

        let hw = w / 2; let hh = h / 2; let tw = w / 3; let th = h / 3;

        let left_half = RECT { left: wa.left, top: wa.top, right: wa.left + hw, bottom: wa.bottom };
        let right_half = RECT { left: wa.left + hw, top: wa.top, right: wa.right, bottom: wa.bottom };
        let tl_q = RECT { left: wa.left, top: wa.top, right: wa.left + hw, bottom: wa.top + hh };
        let tr_q = RECT { left: wa.left + hw, top: wa.top, right: wa.right, bottom: wa.top + hh };
        let bl_q = RECT { left: wa.left, top: wa.top + hh, right: wa.left + hw, bottom: wa.bottom };
        let br_q = RECT { left: wa.left + hw, top: wa.top + hh, right: wa.right, bottom: wa.bottom };

        let left_third = RECT { left: wa.left, top: wa.top, right: wa.left + tw, bottom: wa.bottom };
        let mid_third = RECT { left: wa.left + tw, top: wa.top, right: wa.left + tw*2, bottom: wa.bottom };
        let right_third = RECT { left: wa.left + tw*2, top: wa.top, right: wa.right, bottom: wa.bottom };

        let c1r1 = RECT { left: wa.left, top: wa.top, right: wa.left + tw, bottom: wa.top + th };
        let c2r1 = RECT { left: wa.left + tw, top: wa.top, right: wa.left + tw*2, bottom: wa.top + th };
        let c3r1 = RECT { left: wa.left + tw*2, top: wa.top, right: wa.right, bottom: wa.top + th };
        let c1r2 = RECT { left: wa.left, top: wa.top + th, right: wa.left + tw, bottom: wa.top + th*2 };
        let c2r2 = RECT { left: wa.left + tw, top: wa.top + th, right: wa.left + tw*2, bottom: wa.top + th*2 };
        let c3r2 = RECT { left: wa.left + tw*2, top: wa.top + th, right: wa.right, bottom: wa.top + th*2 };
        let c1r3 = RECT { left: wa.left, top: wa.top + th*2, right: wa.left + tw, bottom: wa.bottom };
        let c2r3 = RECT { left: wa.left + tw, top: wa.top + th*2, right: wa.left + tw*2, bottom: wa.bottom };
        let c3r3 = RECT { left: wa.left + tw*2, top: wa.top + th*2, right: wa.right, bottom: wa.bottom };

        let true_rect = {
            let mut fr = RECT::default();
            if DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut fr as *mut _ as _, size_of::<RECT>() as u32).is_ok() { fr }
            else { let mut r = RECT::default(); let _ = GetWindowRect(hwnd, &mut r); r }
        };

        let is = |r: RECT| {
            (true_rect.left - r.left).abs() <= 25 && (true_rect.right - r.right).abs() <= 25 &&
            (true_rect.top - r.top).abs() <= 25 && (true_rect.bottom - r.bottom).abs() <= 25
        };

        let target_rect = if is_ultrawide {
            match dir {
                SnapDir::Left => {
                    if is(right_third) { mid_third } else if is(mid_third) { left_third }
                    else if is(c3r1) { c2r1 } else if is(c2r1) { c1r1 } else if is(c3r2) { c2r2 } else if is(c2r2) { c1r2 }
                    else if is(c3r3) { c2r3 } else if is(c2r3) { c1r3 } else { left_third }
                },
                SnapDir::Right => {
                    if is(left_third) { mid_third } else if is(mid_third) { right_third }
                    else if is(c1r1) { c2r1 } else if is(c2r1) { c3r1 } else if is(c1r2) { c2r2 } else if is(c2r2) { c3r2 }
                    else if is(c1r3) { c2r3 } else if is(c2r3) { c3r3 } else { right_third }
                },
                SnapDir::Up => {
                    if is(left_third) { c1r1 } else if is(c1r3) { c1r2 } else if is(c1r2) { c1r1 }
                    else if is(mid_third) { c2r1 } else if is(c2r3) { c2r2 } else if is(c2r2) { c2r1 }
                    else if is(right_third) { c3r1 } else if is(c3r3) { c3r2 } else if is(c3r2) { c3r1 } else { wa } 
                },
                SnapDir::Down => {
                    if is(c1r1) { c1r2 } else if is(c1r2) { c1r3 } else if is(left_third) { c1r3 }
                    else if is(c2r1) { c2r2 } else if is(c2r2) { c2r3 } else if is(mid_third) { c2r3 }
                    else if is(c3r1) { c3r2 } else if is(c3r2) { c3r3 } else if is(right_third) { c3r3 }
                    else if is(wa) { let _ = ShowWindowAsync(hwnd, SW_RESTORE); return; }
                    else { let _ = ShowWindowAsync(hwnd, SW_MINIMIZE); return; }
                }
            }
        } else {
            match dir {
                SnapDir::Left => {
                    if is(right_half) { left_half } else if is(tr_q) { tl_q } else if is(br_q) { bl_q } else { left_half }
                },
                SnapDir::Right => {
                    if is(left_half) { right_half } else if is(tl_q) { tr_q } else if is(bl_q) { br_q } else { right_half }
                },
                SnapDir::Up => {
                    if is(left_half) { tl_q } else if is(right_half) { tr_q } else if is(bl_q) { tl_q } else if is(br_q) { tr_q } else { wa }
                },
                SnapDir::Down => {
                    if is(tl_q) { bl_q } else if is(tr_q) { br_q } else if is(left_half) { bl_q } else if is(right_half) { br_q }
                    else if is(wa) { let _ = ShowWindowAsync(hwnd, SW_RESTORE); return; }
                    else { let _ = ShowWindowAsync(hwnd, SW_MINIMIZE); return; }
                }
            }
        };

        apply_snap(hwnd, target_rect);
    }
}

unsafe fn apply_snap(hwnd: HWND, target: RECT) {
    if !(&*addr_of!(SNAPPED_WINDOWS)).contains(&hwnd) {
        let mut r = RECT::default();
        let _ = GetWindowRect(hwnd, &mut r);
        (&mut *addr_of_mut!(PRE_SNAP_RECTS)).retain(|x| x.0 != hwnd);
        (&mut *addr_of_mut!(PRE_SNAP_RECTS)).push((hwnd, r));
        (&mut *addr_of_mut!(SNAPPED_WINDOWS)).push(hwnd);
        
        if (&*addr_of!(PRE_SNAP_RECTS)).len() > 50 { (&mut *addr_of_mut!(PRE_SNAP_RECTS)).remove(0); }
    }

    let mut window_rect = RECT::default(); let mut frame_rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut window_rect);
    let _ = ShowWindowAsync(hwnd, SW_RESTORE); 
    
    if DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut frame_rect as *mut _ as _, size_of::<RECT>() as u32).is_ok() {
        let left_offset = frame_rect.left - window_rect.left; let right_offset = window_rect.right - frame_rect.right;
        let top_offset = frame_rect.top - window_rect.top; let bottom_offset = window_rect.bottom - frame_rect.bottom;

        let final_x = target.left - left_offset; let final_y = target.top - top_offset;
        let final_w = (target.right - target.left) + left_offset + right_offset;
        let final_h = (target.bottom - target.top) + top_offset + bottom_offset;

        let _ = SetWindowPos(hwnd, HWND(0 as _), final_x, final_y, final_w, final_h, SWP_NOZORDER | SWP_ASYNCWINDOWPOS);
    } else {
        let _ = SetWindowPos(hwnd, HWND(0 as _), target.left, target.top, target.right - target.left, target.bottom - target.top, SWP_NOZORDER | SWP_ASYNCWINDOWPOS);
    }
}

unsafe fn calculate_snap_zone(pt: POINT) -> Option<RECT> {
    let mut wa = RECT::default();
    let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut wa as *mut _ as _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
    
    let w = wa.right - wa.left; let h = wa.bottom - wa.top;
    let is_ultrawide = (w as f32 / h as f32) >= 2.3; 
    let trigger = 15; 

    let on_top = pt.y <= wa.top + trigger; let on_bottom = pt.y >= wa.bottom - trigger;
    let on_left = pt.x <= wa.left + trigger; let on_right = pt.x >= wa.right - trigger;

    let hw = w / 2; let hh = h / 2; let tw = w / 3; let th = h / 3;

    if is_ultrawide {
        if on_top && on_left { return Some(RECT { left: wa.left, top: wa.top, right: wa.left + tw, bottom: wa.top + th }); }
        if on_top && on_right { return Some(RECT { left: wa.left + tw*2, top: wa.top, right: wa.right, bottom: wa.top + th }); }
        if on_bottom && on_left { return Some(RECT { left: wa.left, top: wa.top + th*2, right: wa.left + tw, bottom: wa.bottom }); }
        if on_bottom && on_right { return Some(RECT { left: wa.left + tw*2, top: wa.top + th*2, right: wa.right, bottom: wa.bottom }); }

        if on_top {
            if pt.x > wa.left + tw && pt.x < wa.right - tw { return Some(wa); } 
            else if pt.x <= wa.left + tw { return Some(RECT { left: wa.left, top: wa.top, right: wa.left + tw, bottom: wa.top + th }); }
            else { return Some(RECT { left: wa.left + tw*2, top: wa.top, right: wa.right, bottom: wa.top + th }); }
        }
        if on_left { return Some(RECT { left: wa.left, top: wa.top, right: wa.left + tw, bottom: wa.bottom }); }
        if on_right { return Some(RECT { left: wa.left + tw*2, top: wa.top, right: wa.right, bottom: wa.bottom }); }
        if on_bottom {
            if pt.x > wa.left + tw && pt.x < wa.right - tw { return Some(RECT { left: wa.left + tw, top: wa.top + th*2, right: wa.left + tw*2, bottom: wa.bottom }); }
        }
    } else {
        if on_top && on_left { return Some(RECT { left: wa.left, top: wa.top, right: wa.left + hw, bottom: wa.top + hh }); }
        if on_top && on_right { return Some(RECT { left: wa.left + hw, top: wa.top, right: wa.right, bottom: wa.top + hh }); }
        if on_bottom && on_left { return Some(RECT { left: wa.left, top: wa.top + hh, right: wa.left + hw, bottom: wa.bottom }); }
        if on_bottom && on_right { return Some(RECT { left: wa.left + hw, top: wa.top + hh, right: wa.right, bottom: wa.bottom }); }

        if on_top { return Some(wa); }
        if on_left { return Some(RECT { left: wa.left, top: wa.top, right: wa.left + hw, bottom: wa.bottom }); }
        if on_right { return Some(RECT { left: wa.left + hw, top: wa.top, right: wa.right, bottom: wa.bottom }); }
    }
    None
}

unsafe extern "system" fn snap_event_proc(
    _hook: HWINEVENTHOOK, event: u32, hwnd: HWND, id_object: i32, _child: i32, _thread: u32, _time: u32
) {
    if !crate::SNAP_ENABLED { return; }
    if id_object != OBJID_WINDOW.0 as i32 { return; }

    if event == EVENT_SYSTEM_MOVESIZESTART {
        if is_fullscreen(hwnd) { return; }
        DRAGGED_HWND = hwnd;
    } 
    else if event == EVENT_OBJECT_LOCATIONCHANGE && DRAGGED_HWND == hwnd {
        let mut pt = POINT::default(); let _ = GetCursorPos(&mut pt);
        *addr_of_mut!(PENDING_SNAP_RECT) = calculate_snap_zone(pt);
        if let Some(rect) = *addr_of!(PENDING_SNAP_RECT) {
            let _ = SetWindowPos(OVERLAY_HWND, HWND_TOPMOST, rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top, SWP_SHOWWINDOW | SWP_NOACTIVATE);
        } else {
            let _ = ShowWindowAsync(OVERLAY_HWND, SW_HIDE);
        }
    }
    else if event == EVENT_SYSTEM_MOVESIZEEND && DRAGGED_HWND == hwnd {
        let _ = ShowWindowAsync(OVERLAY_HWND, SW_HIDE);
        if let Some(rect) = (&mut *addr_of_mut!(PENDING_SNAP_RECT)).take() {
            apply_snap(hwnd, rect);
        }
        DRAGGED_HWND = HWND(0 as _);
    }
}

unsafe extern "system" fn overlay_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER => {
            if wparam.0 == 1001 {
                if *addr_of!(IS_DRAGGING) { return LRESULT(0); }
                let mut pt = POINT::default(); let _ = GetCursorPos(&mut pt);
                let edges = get_shared_edges();
                let mut found = false;
                
                for edge in edges {
                    let hit = RECT { left: edge.rect.left - 10, top: edge.rect.top - 10, right: edge.rect.right + 10, bottom: edge.rect.bottom + 10 };
                    
                    if pt.x >= hit.left && pt.x <= hit.right && pt.y >= hit.top && pt.y <= hit.bottom {
                        found = true;
                        if (*addr_of!(CURRENT_HOVER_EDGE)).map_or(true, |e| e.w1 != edge.w1 || e.w2 != edge.w2) {
                            *addr_of_mut!(CURRENT_HOVER_EDGE) = Some(edge);
                            let draw_r = if edge.is_vertical {
                                RECT { left: edge.rect.left - 4, top: edge.rect.top, right: edge.rect.right + 4, bottom: edge.rect.bottom }
                            } else {
                                RECT { left: edge.rect.left, top: edge.rect.top - 4, right: edge.rect.right, bottom: edge.rect.bottom + 4 }
                            };
                            let _ = SetWindowPos(RESIZE_LINE_HWND, HWND_TOPMOST, draw_r.left, draw_r.top, draw_r.right - draw_r.left, draw_r.bottom - draw_r.top, SWP_SHOWWINDOW | SWP_NOACTIVATE);
                        }
                        break;
                    }
                }
                if !found && (*addr_of!(CURRENT_HOVER_EDGE)).is_some() {
                    let mut rect = RECT::default(); let _ = GetWindowRect(RESIZE_LINE_HWND, &mut rect);
                    if pt.x < rect.left - 10 || pt.x > rect.right + 10 || pt.y < rect.top - 10 || pt.y > rect.bottom + 10 {
                        *addr_of_mut!(CURRENT_HOVER_EDGE) = None;
                        let _ = ShowWindowAsync(RESIZE_LINE_HWND, SW_HIDE);
                    }
                }
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default(); let hdc = BeginPaint(hwnd, &mut ps);
            let mut rect = RECT::default(); let _ = GetClientRect(hwnd, &mut rect);
            let mut color: u32 = 0; let mut opaque: BOOL = BOOL(0);
            let bg_color = if DwmGetColorizationColor(&mut color, &mut opaque).is_ok() {
                let r = (color >> 16) & 0xFF; let g = (color >> 8) & 0xFF; let b = color & 0xFF; COLORREF(r | (g << 8) | (b << 16))
            } else { COLORREF(0x00FFD700) };
            let brush = CreateSolidBrush(bg_color); FillRect(hdc, &rect, brush); DeleteObject(brush); EndPaint(hwnd, &ps); LRESULT(0)
        }
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as _),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

unsafe extern "system" fn resize_line_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_SETCURSOR => {
            if let Some(edge) = *addr_of!(CURRENT_HOVER_EDGE) {
                let cursor_id = if edge.is_vertical { IDC_SIZEWE } else { IDC_SIZENS };
                let hcursor = LoadCursorW(None, cursor_id).unwrap(); SetCursor(hcursor); return LRESULT(1);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONDOWN => {
            if let Some(edge) = *addr_of!(CURRENT_HOVER_EDGE) {
                let mut pt = POINT::default(); let _ = GetCursorPos(&mut pt);
                *addr_of_mut!(DRAG_START_PT) = pt;
                
                let mut r1 = RECT::default(); let _ = GetWindowRect(edge.w1, &mut r1);
                let mut r2 = RECT::default(); let _ = GetWindowRect(edge.w2, &mut r2);
                *addr_of_mut!(DRAG_W1_RECT) = r1; *addr_of_mut!(DRAG_W2_RECT) = r2;
                
                *addr_of_mut!(IS_DRAGGING) = true; let _ = SetCapture(hwnd);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if *addr_of!(IS_DRAGGING) {
                let mut pt = POINT::default(); let _ = GetCursorPos(&mut pt);
                let dx = pt.x - (*addr_of!(DRAG_START_PT)).x; let dy = pt.y - (*addr_of!(DRAG_START_PT)).y;
                
                if let Some(edge) = *addr_of!(CURRENT_HOVER_EDGE) {
                    let w1_r = *addr_of!(DRAG_W1_RECT); let w2_r = *addr_of!(DRAG_W2_RECT);
                    if let Ok(mut hdwp) = BeginDeferWindowPos(2) {
                        if edge.is_vertical { 
                            let w1_w = (w1_r.right - w1_r.left) + dx; let w2_w = (w2_r.right - w2_r.left) - dx;
                            if let Ok(h) = DeferWindowPos(hdwp, edge.w1, HWND(0 as _), 0, 0, w1_w, w1_r.bottom - w1_r.top, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE) { hdwp = h; }
                            if let Ok(h) = DeferWindowPos(hdwp, edge.w2, HWND(0 as _), w2_r.left + dx, w2_r.top, w2_w, w2_r.bottom - w2_r.top, SWP_NOZORDER | SWP_NOACTIVATE) { hdwp = h; }
                            let _ = SetWindowPos(hwnd, HWND_TOPMOST, edge.rect.left - 4 + dx, edge.rect.top, 8, edge.rect.bottom - edge.rect.top, SWP_NOZORDER | SWP_NOACTIVATE);
                        } else { 
                            let w1_h = (w1_r.bottom - w1_r.top) + dy; let w2_h = (w2_r.bottom - w2_r.top) - dy;
                            if let Ok(h) = DeferWindowPos(hdwp, edge.w1, HWND(0 as _), 0, 0, w1_r.right - w1_r.left, w1_h, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE) { hdwp = h; }
                            if let Ok(h) = DeferWindowPos(hdwp, edge.w2, HWND(0 as _), w2_r.left, w2_r.top + dy, w2_r.right - w2_r.left, w2_h, SWP_NOZORDER | SWP_NOACTIVATE) { hdwp = h; }
                            let _ = SetWindowPos(hwnd, HWND_TOPMOST, edge.rect.left, edge.rect.top - 4 + dy, edge.rect.right - edge.rect.left, 8, SWP_NOZORDER | SWP_NOACTIVATE);
                        }
                        let _ = EndDeferWindowPos(hdwp);
                    }
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if *addr_of!(IS_DRAGGING) {
                *addr_of_mut!(IS_DRAGGING) = false; let _ = ReleaseCapture();
                if let Some(mut edge) = *addr_of!(CURRENT_HOVER_EDGE) {
                    let mut pt = POINT::default(); let _ = GetCursorPos(&mut pt);
                    let dx = pt.x - (*addr_of!(DRAG_START_PT)).x; let dy = pt.y - (*addr_of!(DRAG_START_PT)).y;
                    if edge.is_vertical { edge.rect.left += dx; edge.rect.right += dx; } 
                    else { edge.rect.top += dy; edge.rect.bottom += dy; }
                    *addr_of_mut!(CURRENT_HOVER_EDGE) = Some(edge);
                }
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default(); let hdc = BeginPaint(hwnd, &mut ps);
            let mut rect = RECT::default(); let _ = GetClientRect(hwnd, &mut rect);
            let mut color: u32 = 0; let mut opaque: BOOL = BOOL(0);
            let bg_color = if DwmGetColorizationColor(&mut color, &mut opaque).is_ok() {
                let r = (color >> 16) & 0xFF; let g = (color >> 8) & 0xFF; let b = color & 0xFF; COLORREF(r | (g << 8) | (b << 16))
            } else { COLORREF(0x00FFD700) };
            let brush = CreateSolidBrush(bg_color); FillRect(hdc, &rect, brush); DeleteObject(brush); EndPaint(hwnd, &ps); LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}