use std::ptr::{addr_of, addr_of_mut};
use windows::core::{PCWSTR, w};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use shared::{IDM_ALWAYS_ON_TOP, IDM_TRANSPARENT, IDM_ROLL, IDM_TRAY, WM_ENHANCER_ACTION};

static mut HOOK: HHOOK = HHOOK(0 as _);
static mut MAIN_HWND: HWND = HWND(0 as _);
pub static mut IS_ENABLED: bool = true;
static mut IS_CAPTURING: bool = false;

#[derive(Clone, Default)]
struct HotkeyConfig {
    vk: u16,
    action_type: u32, // 0 = Run, 1 = Keystrokes, 2 = Internal
    action_data: Vec<u16>,
}

static mut CONFIGS: Vec<HotkeyConfig> = Vec::new();
static mut CONFIG_HWND: HWND = HWND(0 as _);
static mut LIST_HWND: HWND = HWND(0 as _);
static mut EDIT_HWND: HWND = HWND(0 as _);
static mut RADIO_RUN_HWND: HWND = HWND(0 as _);
static mut RADIO_KEYS_HWND: HWND = HWND(0 as _);
static mut RADIO_INT_HWND: HWND = HWND(0 as _);
static mut BTN_RECORD_HWND: HWND = HWND(0 as _);

pub struct HotkeyManager;

impl HotkeyManager {
    pub unsafe fn init(hwnd: HWND) {
        *addr_of_mut!(MAIN_HWND) = hwnd;
        Self::load_all();
        let hmod = GetModuleHandleW(None).unwrap();
        *addr_of_mut!(HOOK) = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hmod, 0).unwrap();
    }

    unsafe fn load_all() {
        let mut hkey = HKEY::default();
        let subkey = w!(r"Software\Gallery Inc\IBBE-Hooker\Hotkeys");
        (*addr_of_mut!(CONFIGS)).clear();

        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_READ, &mut hkey).is_ok() {
            // Expanded range: F1 (1) to F24 (24)
            for i in 1..25 {
                let vk_name = format!("F{}\0", i).encode_utf16().collect::<Vec<u16>>();
                let type_name = format!("F{}_Type\0", i).encode_utf16().collect::<Vec<u16>>();
                let mut type_val = 0u32;
                let mut type_size = 4u32;
                let _ = RegQueryValueExW(hkey, PCWSTR(type_name.as_ptr()), None, None, Some(&mut type_val as *mut _ as *mut u8), Some(&mut type_size));

                let mut data = vec![0u8; 512];
                let mut data_size = data.len() as u32;
                if RegQueryValueExW(hkey, PCWSTR(vk_name.as_ptr()), None, None, Some(data.as_mut_ptr()), Some(&mut data_size)).is_ok() {
                    let action_data = std::slice::from_raw_parts(data.as_ptr() as *const u16, (data_size / 2) as usize).to_vec();
                    (*addr_of_mut!(CONFIGS)).push(HotkeyConfig { 
                        vk: (0x6F + i) as u16, 
                        action_type: type_val,
                        action_data 
                    });
                }
            }
            let _ = RegCloseKey(hkey);
        }
    }

    pub unsafe fn show_editor() {
        if (*addr_of!(CONFIG_HWND)).0 != 0 { let _ = SetForegroundWindow(*addr_of!(CONFIG_HWND)); return; }
        let hinstance = GetModuleHandleW(None).unwrap();
        let class_name = w!("HotkeyEditorClass");
        
        let mut wc_check = WNDCLASSW::default();
        if GetClassInfoW(hinstance, class_name, &mut wc_check).is_err() {
            let wc = WNDCLASSW {
                lpfnWndProc: Some(editor_proc), hInstance: hinstance.into(), lpszClassName: class_name,
                hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH((5 + 1) as _), ..Default::default()
            };
            let _ = RegisterClassW(&wc);
        }

        *addr_of_mut!(CONFIG_HWND) = CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW, class_name, w!("Hotkey Manager (F1-F24)"), 
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE, 200, 200, 550, 420, HWND(0 as _), None, hinstance, None);

        let list_style = WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0 | WS_VSCROLL.0 | LBS_NOTIFY as u32);
        *addr_of_mut!(LIST_HWND) = CreateWindowExW(WINDOW_EX_STYLE(0), w!("LISTBOX"), PCWSTR::null(), list_style, 10, 10, 80, 350, *addr_of!(CONFIG_HWND), HMENU(101), hinstance, None);

        for i in 1..25 {
            let name = format!("F{}\0", i).encode_utf16().collect::<Vec<u16>>();
            let _ = SendMessageW(*addr_of!(LIST_HWND), LB_ADDSTRING, WPARAM(0), LPARAM(name.as_ptr() as _));
        }

        let radio_style = WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTORADIOBUTTON as u32);
        *addr_of_mut!(RADIO_RUN_HWND) = CreateWindowExW(WINDOW_EX_STYLE(0), w!("BUTTON"), w!("Run App / URL"), radio_style, 110, 10, 350, 25, *addr_of!(CONFIG_HWND), HMENU(201), hinstance, None);
        *addr_of_mut!(RADIO_KEYS_HWND) = CreateWindowExW(WINDOW_EX_STYLE(0), w!("BUTTON"), w!("Simulate Keys"), radio_style, 110, 40, 350, 25, *addr_of!(CONFIG_HWND), HMENU(202), hinstance, None);
        *addr_of_mut!(RADIO_INT_HWND) = CreateWindowExW(WINDOW_EX_STYLE(0), w!("BUTTON"), w!("Internal App Command"), radio_style, 110, 70, 350, 25, *addr_of!(CONFIG_HWND), HMENU(203), hinstance, None);

        let edit_style = WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32);
        *addr_of_mut!(EDIT_HWND) = CreateWindowExW(WS_EX_CLIENTEDGE, w!("EDIT"), PCWSTR::null(), edit_style, 110, 105, 300, 25, *addr_of!(CONFIG_HWND), HMENU(102), hinstance, None);
        
        let btn_style = WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | BS_PUSHBUTTON as u32);
        *addr_of_mut!(BTN_RECORD_HWND) = CreateWindowExW(WINDOW_EX_STYLE(0), w!("BUTTON"), w!("Record"), btn_style, 420, 105, 80, 25, *addr_of!(CONFIG_HWND), HMENU(301), hinstance, None);
        CreateWindowExW(WINDOW_EX_STYLE(0), w!("BUTTON"), w!("Save Mapping"), btn_style, 110, 150, 120, 35, *addr_of!(CONFIG_HWND), HMENU(103), hinstance, None);
    }

    unsafe fn simulate_keys(input: &str) {
        let parts: Vec<&str> = input.split('+').collect();
        let mut inputs = Vec::new();
        let mut held_keys = Vec::new();
        for part in parts {
            let part = part.trim().to_uppercase();
            let vk = match part.as_str() {
                "CTRL" | "CONTROL" => VK_LCONTROL, "ALT" | "MENU" => VK_LMENU, "SHIFT" => VK_LSHIFT, "WIN" | "LWIN" => VK_LWIN,
                "ENTER" => VK_RETURN, "ESC" | "ESCAPE" => VK_ESCAPE, "SPACE" => VK_SPACE, "TAB" => VK_TAB,
                s if s.len() == 1 => VIRTUAL_KEY(s.as_bytes()[0] as u16),
                s if s.starts_with('F') => { if let Ok(num) = s[1..].parse::<u16>() { VIRTUAL_KEY(0x6F + num) } else { VIRTUAL_KEY(0) } },
                _ => VIRTUAL_KEY(0),
            };
            if vk.0 != 0 { held_keys.push(vk); let mut inp = INPUT { r#type: INPUT_KEYBOARD, ..Default::default() }; inp.Anonymous.ki.wVk = vk; inputs.push(inp); }
        }
        for vk in held_keys.iter().rev() { let mut inp = INPUT { r#type: INPUT_KEYBOARD, ..Default::default() }; inp.Anonymous.ki.wVk = *vk; inp.Anonymous.ki.dwFlags = KEYEVENTF_KEYUP; inputs.push(inp); }
        if !inputs.is_empty() { let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32); }
    }

    pub unsafe fn cleanup() { if (*addr_of!(HOOK)).0 != 0 { let _ = UnhookWindowsHookEx(*addr_of!(HOOK)); } }
}

unsafe extern "system" fn editor_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as u32;
            if id == 101 && (wparam.0 >> 16) as u32 == LBN_SELCHANGE {
                let idx = SendMessageW(*addr_of!(LIST_HWND), LB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as i32;
                let vk = (0x70 + idx) as u16;
                let cfg = (*addr_of!(CONFIGS)).iter().find(|c| c.vk == vk);
                let (act_type, act_data) = cfg.map(|c| (c.action_type, c.action_data.clone())).unwrap_or((0, vec![0u16]));
                let _ = SetWindowTextW(*addr_of!(EDIT_HWND), PCWSTR(act_data.as_ptr()));
                SendMessageW(*addr_of!(RADIO_RUN_HWND), BM_SETCHECK, WPARAM((act_type == 0) as usize), LPARAM(0));
                SendMessageW(*addr_of!(RADIO_KEYS_HWND), BM_SETCHECK, WPARAM((act_type == 1) as usize), LPARAM(0));
                SendMessageW(*addr_of!(RADIO_INT_HWND), BM_SETCHECK, WPARAM((act_type == 2) as usize), LPARAM(0));
            } else if id == 301 { *addr_of_mut!(IS_CAPTURING) = true; let _ = SetWindowTextW(*addr_of!(EDIT_HWND), w!("Recording...")); }
            else if id == 103 {
                let idx = SendMessageW(*addr_of!(LIST_HWND), LB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as i32;
                if idx < 0 { return LRESULT(0); }
                let mut buf = vec![0u16; 512]; let len = GetWindowTextW(*addr_of!(EDIT_HWND), &mut buf);
                buf.truncate(len as usize); buf.push(0);
                let act_type = if SendMessageW(*addr_of!(RADIO_INT_HWND), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1 { 2 }
                              else if SendMessageW(*addr_of!(RADIO_KEYS_HWND), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1 { 1 } else { 0 };
                let mut hkey = HKEY::default();
                if RegCreateKeyExW(HKEY_CURRENT_USER, w!(r"Software\Gallery Inc\IBBE-Hooker\Hotkeys"), 0, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_ALL_ACCESS, None, &mut hkey, None).is_ok() {
                    let _ = RegSetValueExW(hkey, PCWSTR(format!("F{}\0", idx + 1).encode_utf16().collect::<Vec<u16>>().as_ptr()), 0, REG_SZ, Some(std::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 2)));
                    let _ = RegSetValueExW(hkey, PCWSTR(format!("F{}_Type\0", idx + 1).encode_utf16().collect::<Vec<u16>>().as_ptr()), 0, REG_DWORD, Some(std::slice::from_raw_parts(&act_type as *const _ as *const u8, 4)));
                    let _ = RegCloseKey(hkey);
                }
                HotkeyManager::load_all();
            }
            LRESULT(0)
        }
        WM_CLOSE => { let _ = DestroyWindow(hwnd); LRESULT(0) }
        WM_DESTROY => { *addr_of_mut!(CONFIG_HWND) = HWND(0 as _); LRESULT(0) }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && *addr_of!(IS_ENABLED) {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        if (kb.flags.0 & LLKHF_INJECTED.0) != 0 { return CallNextHookEx(*addr_of!(HOOK), code, wparam, lparam); }
        let is_down = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;
        
        if *addr_of!(IS_CAPTURING) && is_down {
            let mut hotkey_str = String::new();
            if GetAsyncKeyState(VK_CONTROL.0 as i32) < 0 { hotkey_str.push_str("CTRL+"); }
            if GetAsyncKeyState(VK_MENU.0 as i32) < 0 { hotkey_str.push_str("ALT+"); }
            if GetAsyncKeyState(VK_SHIFT.0 as i32) < 0 { hotkey_str.push_str("SHIFT+"); }
            if GetAsyncKeyState(VK_LWIN.0 as i32) < 0 || GetAsyncKeyState(VK_RWIN.0 as i32) < 0 { hotkey_str.push_str("WIN+"); }
            let vk = kb.vkCode as u16;
            if ![16, 17, 18, 91, 92].contains(&vk) {
                let mut scan_code = kb.scanCode << 16; if kb.flags.0 & LLKHF_EXTENDED.0 != 0 { scan_code |= 1 << 24; }
                let mut buf = [0u16; 64]; let len = GetKeyNameTextW(scan_code as i32, &mut buf);
                if len > 0 { hotkey_str.push_str(&String::from_utf16_lossy(&buf[..len as usize]).to_uppercase()); } else { hotkey_str.push_str("KEY"); }
                let result_w: Vec<u16> = (hotkey_str + "\0").encode_utf16().collect();
                let _ = SetWindowTextW(*addr_of!(EDIT_HWND), PCWSTR(result_w.as_ptr()));
                *addr_of_mut!(IS_CAPTURING) = false;
                SendMessageW(*addr_of!(RADIO_KEYS_HWND), BM_SETCHECK, WPARAM(1), LPARAM(0));
                SendMessageW(*addr_of!(RADIO_RUN_HWND), BM_SETCHECK, WPARAM(0), LPARAM(0));
            }
            return LRESULT(1);
        }

        if is_down {
            let vk = kb.vkCode as u16;
            if (0x70..=0x87).contains(&vk) { // Intercept F1 to F24
                if let Some(cfg) = (*addr_of!(CONFIGS)).iter().find(|c| c.vk == vk) {
                    let cmd_str = String::from_utf16_lossy(&cfg.action_data).trim_matches('\0').to_uppercase();
                    if !cmd_str.is_empty() {
                        match cfg.action_type {
                            0 => { let _ = ShellExecuteW(HWND(0 as _), w!("open"), PCWSTR(cfg.action_data.as_ptr()), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL); }
                            1 => { HotkeyManager::simulate_keys(&cmd_str); }
                            2 => {
                                let target = GetForegroundWindow();
                                let action = match cmd_str.as_str() { "TOGGLE_TOP" => IDM_ALWAYS_ON_TOP, "TOGGLE_TRANSP" => IDM_TRANSPARENT, "ROLL_UP" => IDM_ROLL, "SEND_TRAY" => IDM_TRAY, _ => 0 };
                                if action != 0 { PostMessageW(*addr_of!(MAIN_HWND), WM_ENHANCER_ACTION, WPARAM(action as usize), LPARAM(target.0 as _)); }
                            }
                            _ => {}
                        }
                        return LRESULT(1); 
                    }
                }
            }
        }
    }
    CallNextHookEx(*addr_of!(HOOK), code, wparam, lparam)
}