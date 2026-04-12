use std::ptr::{addr_of, addr_of_mut};
use windows::core::w;
use windows::Win32::Foundation::{BOOL, COLORREF, HANDLE, HGLOBAL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::DataExchange::{OpenClipboard, GetClipboardData, SetClipboardData, EmptyClipboard, CloseClipboard, IsClipboardFormatAvailable};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// Definim manual constantele de sistem care pot lipsi din modulele standard
const WM_MOUSELEAVE: u32 = 0x02A3;
const CF_UNICODETEXT: u32 = 13;

static mut PASTE_HWND: HWND = HWND(0 as _);
static mut HOVER_INDEX: i32 = -1;

pub struct AdvancedPasteManager;

impl AdvancedPasteManager {
    pub unsafe fn init() {
        let h_inst = GetModuleHandleW(None).unwrap();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(paste_wnd_proc),
            hInstance: h_inst.into(),
            lpszClassName: w!("AdvancedPasteClass"),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);

        PASTE_HWND = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            w!("AdvancedPasteClass"), w!("AdvancedPaste"), WS_POPUP,
            0, 0, 260, 150, HWND(0 as _), None, h_inst, None
        );

        let _ = SetLayeredWindowAttributes(PASTE_HWND, COLORREF(0), 240, LWA_ALPHA);
    }

    pub unsafe fn show() {
        if IsWindowVisible(PASTE_HWND).as_bool() {
            let _ = ShowWindowAsync(PASTE_HWND, SW_HIDE);
            return;
        }

        // REZOLVARE EROARE: IsClipboardFormatAvailable returnează Result în v0.52
        if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() {
            return; 
        }

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);

        let _ = SetWindowPos(PASTE_HWND, HWND_TOPMOST, pt.x + 15, pt.y + 15, 260, 150, SWP_SHOWWINDOW | SWP_NOACTIVATE);
        *addr_of_mut!(HOVER_INDEX) = -1;
        let _ = InvalidateRect(PASTE_HWND, None, BOOL(1));
    }

    pub unsafe fn hide() {
        let _ = ShowWindowAsync(PASTE_HWND, SW_HIDE);
    }
}

unsafe fn get_clipboard_text() -> Option<String> {
    if OpenClipboard(HWND(0 as _)).is_ok() {
        if let Ok(handle) = GetClipboardData(CF_UNICODETEXT) {
            let hglobal = HGLOBAL(handle.0 as _);
            let ptr = GlobalLock(hglobal);
            if !ptr.is_null() {
                let slice = std::slice::from_raw_parts(ptr as *const u16, 0xFFFFFF);
                let len = slice.iter().position(|&c| c == 0).unwrap_or(0);
                let text = String::from_utf16_lossy(&slice[..len]);
                let _ = GlobalUnlock(hglobal);
                let _ = CloseClipboard();
                return Some(text);
            }
        }
        let _ = CloseClipboard();
    }
    None
}

unsafe fn set_clipboard_text(text: &str) {
    if OpenClipboard(HWND(0 as _)).is_ok() {
        let _ = EmptyClipboard();
        let mut encoded: Vec<u16> = text.encode_utf16().collect();
        encoded.push(0);

        if let Ok(mem) = GlobalAlloc(GMEM_MOVEABLE, encoded.len() * 2) {
            let ptr = GlobalLock(mem);
            if !ptr.is_null() {
                std::ptr::copy_nonoverlapping(encoded.as_ptr(), ptr as *mut u16, encoded.len());
                let _ = GlobalUnlock(mem);
                let _ = SetClipboardData(CF_UNICODETEXT, HANDLE(mem.0 as _));
            }
        }
        let _ = CloseClipboard();
    }
}

unsafe fn execute_paste(format: i32) {
    if let Some(text) = get_clipboard_text() {
        let new_text = match format {
            0 => text, 
            1 => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                    serde_json::to_string_pretty(&parsed).unwrap_or(text) 
                } else {
                    serde_json::to_string(&text).unwrap_or(text) 
                }
            },
            2 => {
                if text.contains('\n') {
                    format!("```text\n{}\n```", text) 
                } else {
                    format!("> {}", text) 
                }
            },
            _ => text,
        };

        set_clipboard_text(&new_text);
        
        std::thread::sleep(std::time::Duration::from_millis(50));

        let mut inputs = [INPUT::default(); 4];
        inputs[0].r#type = INPUT_KEYBOARD;
        inputs[0].Anonymous.ki.wVk = VK_CONTROL;
        inputs[1].r#type = INPUT_KEYBOARD;
        inputs[1].Anonymous.ki.wVk = VIRTUAL_KEY(0x56); 
        inputs[2].r#type = INPUT_KEYBOARD;
        inputs[2].Anonymous.ki.wVk = VIRTUAL_KEY(0x56); 
        inputs[2].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
        inputs[3].r#type = INPUT_KEYBOARD;
        inputs[3].Anonymous.ki.wVk = VK_CONTROL;
        inputs[3].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

unsafe extern "system" fn paste_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as _), 
        WM_MOUSEMOVE => {
            let y = (lparam.0 >> 16) as i16 as i32;
            let new_hover = y / 50;
            if new_hover != *addr_of!(HOVER_INDEX) && new_hover >= 0 && new_hover < 3 {
                *addr_of_mut!(HOVER_INDEX) = new_hover;
                let _ = InvalidateRect(hwnd, None, BOOL(1));
                
                let mut tme = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
                let _ = TrackMouseEvent(&mut tme);
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            *addr_of_mut!(HOVER_INDEX) = -1;
            let _ = InvalidateRect(hwnd, None, BOOL(1));
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let y = (lparam.0 >> 16) as i16 as i32;
            let index = y / 50;
            if index >= 0 && index < 3 {
                AdvancedPasteManager::hide();
                execute_paste(index);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);

            let bg_brush = CreateSolidBrush(COLORREF(0x001A1A1A));
            FillRect(hdc, &rect, bg_brush); DeleteObject(bg_brush);

            let border_brush = CreateSolidBrush(COLORREF(0x00404040));
            FrameRect(hdc, &rect, border_brush); DeleteObject(border_brush);

            let _ = SetBkMode(hdc, TRANSPARENT as _);
            
            let font = CreateFontW(
                20, 0, 0, 0, 400, 0, 0, 0, DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32, CLIP_DEFAULT_PRECIS.0 as u32,
                CLEARTYPE_QUALITY.0 as u32, DEFAULT_PITCH.0 as u32, w!("Segoe UI")
            );
            let old_font = SelectObject(hdc, font);

            let items = [
                w!("📋  Paste as Plain Text"),
                w!("{ }  Paste as JSON"),
                w!("Ⓜ  Paste as Markdown"),
            ];

            for i in 0..3 {
                let mut item_rect = RECT { left: 2, top: i * 50 + 2, right: rect.right - 2, bottom: (i + 1) * 50 - 2 };
                
                if i == *addr_of!(HOVER_INDEX) {
                    let hover_brush = CreateSolidBrush(COLORREF(0x00333333));
                    FillRect(hdc, &item_rect, hover_brush); DeleteObject(hover_brush);
                }

                let _ = SetTextColor(hdc, COLORREF(0x00FFFFFF));
                item_rect.left += 15; item_rect.top += 12;
                let mut text = items[i as usize].as_wide().to_vec();
                let _ = DrawTextW(hdc, &mut text, &mut item_rect, DT_LEFT | DT_TOP | DT_SINGLELINE);
            }

            SelectObject(hdc, old_font); DeleteObject(font);
            EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}