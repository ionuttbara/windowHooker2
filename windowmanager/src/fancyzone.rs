use std::ptr::{addr_of, addr_of_mut};
use windows::Win32::System::Registry::{RegCreateKeyExW, RegCloseKey, RegSetValueExW, RegEnumValueW, RegDeleteKeyW, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_ALL_ACCESS, KEY_READ, REG_OPTION_NON_VOLATILE, REG_BINARY};
use windows::core::{PCWSTR, PWSTR, w};
use windows::Win32::Foundation::{POINT, RECT, HWND, BOOL, COLORREF, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_RETURN, GetAsyncKeyState, VK_SHIFT, VK_F1};

pub static mut FANCY_ZONES_ENABLED: bool = false;
static mut EDITOR_HWND: HWND = HWND(0 as _);
static mut TEMP_ZONES: Vec<RECT> = Vec::new();
static mut CURRENT_DRAW: Option<RECT> = None;
static mut DRAW_START: POINT = POINT { x: 0, y: 0 };
static mut IS_DRAWING: bool = false;
static mut EDITOR_LANG: u32 = 0;

static mut GLOBAL_MANAGER: Option<FancyZoneManager> = None;

#[derive(Clone, Copy)]
pub struct FancyZone {
    pub rect: RECT,
}

pub struct FancyZoneManager {
    pub zones: Vec<FancyZone>,
}

impl FancyZoneManager {
    pub fn new() -> Self {
        let mut fz = Self { zones: Vec::new() };
        fz.load_from_registry();
        unsafe { 
            GLOBAL_MANAGER = Some(Self { zones: fz.zones.clone() }); 
            crate::winnsnap::FANCYZONE_CALLBACK = Some(Self::get_zone_for_point);
        }
        fz
    }

    pub fn open_editor() {
        unsafe {
            if let Some(ref mut mgr) = GLOBAL_MANAGER {
                *addr_of_mut!(TEMP_ZONES) = mgr.zones.iter().map(|z| z.rect).collect();
            } else {
                *addr_of_mut!(TEMP_ZONES) = vec![];
            }
            
            // Read language for translations
            let mut lang_id = 0u32;
            let mut hkey = windows::Win32::System::Registry::HKEY::default();
            if RegOpenKeyExW(HKEY_CURRENT_USER, w!("Software\\Gallery Inc\\IBBE-Hooker"), 0, KEY_READ, &mut hkey).is_ok() {
                let mut size = 4u32;
                let _ = RegQueryValueExW(hkey, w!("Language"), None, None, Some(&mut lang_id as *mut _ as *mut u8), Some(&mut size));
                let _ = RegCloseKey(hkey);
            }
            EDITOR_LANG = lang_id;

            let hinstance = GetModuleHandleW(None).unwrap();
            let wc = WNDCLASSW { 
                lpfnWndProc: Some(editor_proc), 
                hInstance: hinstance.into(), 
                lpszClassName: w!("FancyZoneEditorClass"), 
                hCursor: LoadCursorW(None, IDC_CROSS).unwrap(),
                ..Default::default() 
            };
            let _ = RegisterClassW(&wc);

            let sw = GetSystemMetrics(SM_CXSCREEN);
            let sh = GetSystemMetrics(SM_CYSCREEN);
            
            let title = shared::tr_w(lang_id, "fz_editor_title");

            EDITOR_HWND = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                w!("FancyZoneEditorClass"), PCWSTR(title.as_ptr()), WS_POPUP,
                0, 0, sw, sh, HWND(0 as _), None, hinstance, None
            );
            
            let _ = SetLayeredWindowAttributes(EDITOR_HWND, COLORREF(0), 180, LWA_ALPHA);
            let _ = ShowWindowAsync(EDITOR_HWND, SW_SHOW);
            let _ = SetForegroundWindow(EDITOR_HWND);
        }
    }

    fn load_from_registry(&mut self) {
        unsafe {
            let mut hkey = windows::Win32::System::Registry::HKEY::default();
            let subkey = w!("Software\\Gallery Inc\\IBBE-Hooker\\FancyZones");
            
            self.zones.clear();
            if RegCreateKeyExW(HKEY_CURRENT_USER, subkey, 0, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_READ | KEY_ALL_ACCESS, None, &mut hkey, None).is_ok() {
                let mut index = 0;
                loop {
                    let mut name = vec![0u16; 256];
                    let mut name_len = name.len() as u32;
                    let mut data = vec![0u8; std::mem::size_of::<RECT>()];
                    let mut data_len = data.len() as u32;
                    
                    let res = RegEnumValueW(
                        hkey, index, PWSTR(name.as_mut_ptr()), &mut name_len, None, None, Some(data.as_mut_ptr()), Some(&mut data_len)
                    );
                    
                    if res.is_err() { break; } 
                    
                    if data_len == std::mem::size_of::<RECT>() as u32 {
                        let rect = *(data.as_ptr() as *const RECT);
                        self.zones.push(FancyZone { rect });
                    }
                    index += 1;
                }
                let _ = RegCloseKey(hkey);
            }
            
            if self.zones.is_empty() {
                self.zones.push(FancyZone { rect: RECT { left: 0, top: 0, right: 800, bottom: 1080 }});
                self.zones.push(FancyZone { rect: RECT { left: 800, top: 0, right: 1920, bottom: 1080 }});
            }
        }
    }

    pub fn save_to_registry(rects: &Vec<RECT>) {
        unsafe {
            let subkey = w!("Software\\Gallery Inc\\IBBE-Hooker\\FancyZones");
            
            let _ = RegDeleteKeyW(HKEY_CURRENT_USER, subkey);
            
            let mut hkey = windows::Win32::System::Registry::HKEY::default();
            if RegCreateKeyExW(HKEY_CURRENT_USER, subkey, 0, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_ALL_ACCESS, None, &mut hkey, None).is_ok() {
                for (i, r) in rects.iter().enumerate() {
                    let name_str = format!("{:02}_zone\0", i + 1);
                    let name_w: Vec<u16> = name_str.encode_utf16().collect();
                    
                    let data_ptr = r as *const RECT as *const u8;
                    let data_size = std::mem::size_of::<RECT>() as u32;
                    
                    let _ = RegSetValueExW(hkey, PCWSTR(name_w.as_ptr()), 0, REG_BINARY, Some(std::slice::from_raw_parts(data_ptr, data_size as usize)));
                }
                let _ = RegCloseKey(hkey);
            }
            
            if let Some(ref mut mgr) = GLOBAL_MANAGER {
                mgr.load_from_registry();
            }
        }
    }

    pub fn get_zone_for_point(pt: windows::Win32::Foundation::POINT) -> Option<RECT> {
        unsafe {
            if !FANCY_ZONES_ENABLED { return None; }
            
            // Permite snap catre zone DOAR cand sunt apasate concomitent SHIFT si F1
            let shift_pressed = GetAsyncKeyState(VK_SHIFT.0 as i32) < 0;
            let f1_pressed = GetAsyncKeyState(VK_F1.0 as i32) < 0;
            if !(shift_pressed && f1_pressed) {
                return None;
            }

            if let Some(ref mgr) = GLOBAL_MANAGER {
                for zone in &mgr.zones {
                    if pt.x >= zone.rect.left && pt.x <= zone.rect.right && pt.y >= zone.rect.top && pt.y <= zone.rect.bottom {
                        return Some(zone.rect);
                    }
                }
            }
        }
        None
    }

    pub fn snap_active_to_zone(zone_index: usize) {
        unsafe {
            if !FANCY_ZONES_ENABLED { return; }
            if let Some(ref mgr) = GLOBAL_MANAGER {
                if zone_index < mgr.zones.len() {
                    let target_rect = mgr.zones[zone_index].rect;
                    let hwnd = GetForegroundWindow();
                    if hwnd.0 != 0 {
                        crate::winnsnap::apply_snap(hwnd, target_rect);
                    }
                }
            }
        }
    }
}

unsafe extern "system" fn editor_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_KEYDOWN => {
            if wparam.0 == VK_ESCAPE.0 as usize || wparam.0 == VK_RETURN.0 as usize {
                FancyZoneManager::save_to_registry(&*addr_of!(TEMP_ZONES));
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let mut pt = POINT::default(); let _ = GetCursorPos(&mut pt);
            *addr_of_mut!(DRAW_START) = pt;
            *addr_of_mut!(IS_DRAWING) = true;
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if *addr_of!(IS_DRAWING) {
                let mut pt = POINT::default(); let _ = GetCursorPos(&mut pt);
                let start = *addr_of!(DRAW_START);
                *addr_of_mut!(CURRENT_DRAW) = Some(RECT {
                    left: start.x.min(pt.x), top: start.y.min(pt.y),
                    right: start.x.max(pt.x), bottom: start.y.max(pt.y),
                });
                let _ = InvalidateRect(hwnd, None, BOOL(0)); // Apeleaza cu FALSE pt a folosi paint-ul propriu fara erase automat
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if *addr_of!(IS_DRAWING) {
                *addr_of_mut!(IS_DRAWING) = false;
                if let Some(r) = *addr_of!(CURRENT_DRAW) {
                    if r.right - r.left > 50 && r.bottom - r.top > 50 {
                        (*addr_of_mut!(TEMP_ZONES)).push(r);
                    }
                }
                *addr_of_mut!(CURRENT_DRAW) = None;
                let _ = InvalidateRect(hwnd, None, BOOL(0));
            }
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            let mut pt = POINT::default(); let _ = GetCursorPos(&mut pt);
            (*addr_of_mut!(TEMP_ZONES)).retain(|z| !(pt.x >= z.left && pt.x <= z.right && pt.y >= z.top && pt.y <= z.bottom));
            let _ = InvalidateRect(hwnd, None, BOOL(0));
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1), // --- FIX FLICKERING: Nu permite Windows sa stearga fundalul nativ ---
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default(); let hdc = BeginPaint(hwnd, &mut ps);
            let mut rect = RECT::default(); let _ = GetClientRect(hwnd, &mut rect);
            
            // --- FIX FLICKERING: Double Buffering ---
            let mem_dc = CreateCompatibleDC(hdc);
            let mem_bitmap = CreateCompatibleBitmap(hdc, rect.right - rect.left, rect.bottom - rect.top);
            let old_bitmap = SelectObject(mem_dc, mem_bitmap);

            // Fundal negru transparent
            let bg_brush = CreateSolidBrush(COLORREF(0x00000000));
            FillRect(mem_dc, &rect, bg_brush);
            DeleteObject(bg_brush);

            // --- INSTRUCTIUNI ---
            let hfont_instr = CreateFontW(24, 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0, DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32, CLIP_DEFAULT_PRECIS.0 as u32, DEFAULT_QUALITY.0 as u32, DEFAULT_PITCH.0 as u32, w!("Segoe UI"));
            let old_font_instr = SelectObject(mem_dc, hfont_instr);
            SetBkMode(mem_dc, TRANSPARENT);
            SetTextColor(mem_dc, COLORREF(0x00FFFFFF));
            
            let lang = *addr_of!(EDITOR_LANG);
            let instr_text = if lang == 0 { "Click-Stanga/Trage: Creare | Click-Dreapta: Stergere Zona | Esc/Enter: Salvare\0" } 
                             else if lang == 1 { "Left-Click/Drag: Create | Right-Click: Remove Zone | Esc/Enter: Save\0" } 
                             else { "Bal-Klikk/Huz: Letrehozas | Jobb-Klikk: Torles | Esc/Enter: Mentes\0" };
            
            let mut instr_w: Vec<u16> = instr_text.encode_utf16().collect();
            let mut instr_rect = RECT { left: 20, top: 20, right: rect.right, bottom: 60 };
            let _ = DrawTextW(mem_dc, &mut instr_w[..], &mut instr_rect, DT_LEFT | DT_TOP | DT_SINGLELINE);
            SelectObject(mem_dc, old_font_instr);
            DeleteObject(hfont_instr);

            // --- DESENARE ZONE ---
            let brush_zone = CreateSolidBrush(COLORREF(0x00D7AA00)); 
            
            // --- FIX FONT MARIT ---
            let hfont_zone = CreateFontW(64, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32, CLIP_DEFAULT_PRECIS.0 as u32, DEFAULT_QUALITY.0 as u32, DEFAULT_PITCH.0 as u32, w!("Segoe UI"));
            let old_font_zone = SelectObject(mem_dc, hfont_zone);

            for (i, z) in (*addr_of!(TEMP_ZONES)).iter().enumerate() {
                FillRect(mem_dc, z, brush_zone);
                
                SetBkMode(mem_dc, TRANSPARENT);
                SetTextColor(mem_dc, COLORREF(0x00FFFFFF));
                
                let text_base = shared::tr_w(lang, "fz_zone_label");
                let base_str = String::from_utf16_lossy(&text_base[..text_base.len()-1]); // without null terminator
                let text = format!("{} {}\0", base_str, i + 1);
                
                let mut text_w: Vec<u16> = text.encode_utf16().collect();
                let mut text_rect = *z;
                let _ = DrawTextW(mem_dc, &mut text_w[..], &mut text_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            }
            
            SelectObject(mem_dc, old_font_zone);
            DeleteObject(hfont_zone);
            
            if let Some(ref z) = *addr_of!(CURRENT_DRAW) {
                let brush_draw = CreateSolidBrush(COLORREF(0x0000A5FF)); 
                FillRect(mem_dc, z, brush_draw);
                DeleteObject(brush_draw);
            }
            
            DeleteObject(brush_zone);

            // Copiaza din memorie direct pe ecran pentru a evita intermitenta
            let _ = BitBlt(hdc, 0, 0, rect.right - rect.left, rect.bottom - rect.top, mem_dc, 0, 0, SRCCOPY);

            SelectObject(mem_dc, old_bitmap);
            DeleteObject(mem_bitmap);
            DeleteDC(mem_dc);

            EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}