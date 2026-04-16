use std::ptr::{addr_of, addr_of_mut};
use windows::core::{PCWSTR, w};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM, BOOL};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Registry::{RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_READ};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

static mut HOOK: HHOOK = HHOOK(0 as _);
static mut MAIN_HWND: HWND = HWND(0 as _);
pub static mut IS_ENABLED: bool = true;

#[derive(PartialEq, Clone, Copy)]
enum State { Idle, LWin, LShift, F23 }

static mut PRESS_STATE: State = State::Idle;
static mut RELEASE_STATE: State = State::Idle;
static mut LWIN_SUPPRESSED: bool = false;
static mut LSHIFT_SUPPRESSED: bool = false;

// 0 = Key Inject (default RControl), 1 = Run App/URL
static mut ACTION_TYPE: u32 = 0; 
static mut ACTION_DATA: Vec<u16> = Vec::new();

// UI pt config
static mut CONFIG_HWND: HWND = HWND(0 as _);
static mut EDIT_HWND: HWND = HWND(0 as _);

pub struct NoCopilotManager;

impl NoCopilotManager {
    pub unsafe fn init(hwnd: HWND) {
        MAIN_HWND = hwnd;
        Self::load_config();
        let hmod = GetModuleHandleW(None).unwrap();
        HOOK = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hmod, 0).unwrap();
    }

    unsafe fn load_config() {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        let subkey = w!("Software\\Gallery Inc\\IBBE-Hooker\\NoCopilot");
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_READ, &mut hkey).is_ok() {
            let mut act_type: u32 = 0; let mut type_size = 4;
            if RegQueryValueExW(hkey, w!("ActionType"), None, None, Some(&mut act_type as *mut _ as _), Some(&mut type_size)).is_ok() {
                *addr_of_mut!(ACTION_TYPE) = act_type;
            }
            
            let mut data = vec![0u8; 512]; let mut data_size = data.len() as u32;
            if RegQueryValueExW(hkey, w!("ActionData"), None, None, Some(data.as_mut_ptr()), Some(&mut data_size)).is_ok() {
                let u16_slice: &[u16] = std::slice::from_raw_parts(data.as_ptr() as *const u16, (data_size / 2) as usize);
                *addr_of_mut!(ACTION_DATA) = u16_slice.to_vec();
            }
            let _ = windows::Win32::System::Registry::RegCloseKey(hkey);
        } else {
            *addr_of_mut!(ACTION_TYPE) = 0;
            *addr_of_mut!(ACTION_DATA) = w!("VK_RCONTROL").as_wide().to_vec();
        }
    }

    pub unsafe fn show_config_dialog() {
        if CONFIG_HWND.0 != 0 {
            let _ = SetForegroundWindow(CONFIG_HWND);
            return;
        }
        
        let mut lang_id = 0u32;
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, w!("Software\\Gallery Inc\\IBBE-Hooker"), 0, KEY_READ, &mut hkey).is_ok() {
            let mut size = 4u32;
            let _ = RegQueryValueExW(hkey, w!("Language"), None, None, Some(&mut lang_id as *mut _ as *mut u8), Some(&mut size));
            let _ = windows::Win32::System::Registry::RegCloseKey(hkey);
        }

        let title = shared::tr_w(lang_id, "cfg_title");
        let radio1 = shared::tr_w(lang_id, "cfg_radio1");
        let radio2 = shared::tr_w(lang_id, "cfg_radio2");
        let btn_save = shared::tr_w(lang_id, "cfg_save");

        let hinstance = GetModuleHandleW(None).unwrap();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(config_proc), 
            hInstance: hinstance.into(), 
            lpszClassName: w!("CopilotConfigClass"), 
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH((5 + 1) as _), 
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);
        
        let width = 420; let height = 200;
        let sw = GetSystemMetrics(SM_CXSCREEN); let sh = GetSystemMetrics(SM_CYSCREEN);
        
        CONFIG_HWND = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST, 
            w!("CopilotConfigClass"), 
            PCWSTR(title.as_ptr()), 
            WS_POPUP | WS_CAPTION | WS_SYSMENU, 
            (sw - width) / 2, (sh - height) / 2, 
            width, height, 
            HWND(0 as _), None, hinstance, None
        );
        
        let btn_style = WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTORADIOBUTTON as u32);
        let push_style = WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | BS_PUSHBUTTON as u32);
        let edit_style = WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32);

        CreateWindowExW(WINDOW_EX_STYLE(0), w!("BUTTON"), PCWSTR(radio1.as_ptr()), btn_style, 20, 20, 350, 20, CONFIG_HWND, HMENU(1), hinstance, None);
        CreateWindowExW(WINDOW_EX_STYLE(0), w!("BUTTON"), PCWSTR(radio2.as_ptr()), btn_style, 20, 50, 350, 20, CONFIG_HWND, HMENU(2), hinstance, None);
        EDIT_HWND = CreateWindowExW(WS_EX_CLIENTEDGE, w!("EDIT"), PCWSTR::null(), edit_style, 40, 80, 340, 25, CONFIG_HWND, HMENU(3), hinstance, None);
        CreateWindowExW(WINDOW_EX_STYLE(0), w!("BUTTON"), PCWSTR(btn_save.as_ptr()), push_style, 160, 120, 100, 30, CONFIG_HWND, HMENU(4), hinstance, None);
        
        if *addr_of!(ACTION_TYPE) == 0 {
            SendDlgItemMessageW(CONFIG_HWND, 1, 0x00F1 /*BM_SETCHECK*/, WPARAM(1), LPARAM(0));
            let _ = EnableWindow(EDIT_HWND, BOOL(0));
        } else {
            SendDlgItemMessageW(CONFIG_HWND, 2, 0x00F1, WPARAM(1), LPARAM(0));
            let _ = EnableWindow(EDIT_HWND, BOOL(1));
            // --- FIX PENTRU WARNING-URI ---
            let _ = SetWindowTextW(EDIT_HWND, PCWSTR((*addr_of!(ACTION_DATA)).as_ptr()));
        }
        
        let _ = ShowWindow(CONFIG_HWND, SW_SHOW);
    }

    pub unsafe fn execute_action() {
        // --- FIX PENTRU WARNING-URI ---
        if *addr_of!(ACTION_TYPE) == 1 && !(*addr_of!(ACTION_DATA)).is_empty() {
            let _ = ShellExecuteW(HWND(0 as _), w!("open"), PCWSTR((*addr_of!(ACTION_DATA)).as_ptr()), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
        } else {
            Self::inject_key(VK_RCONTROL.0, false);
        }
    }

    pub unsafe fn finish_action() {
        if *addr_of!(ACTION_TYPE) == 0 {
            Self::inject_key(VK_RCONTROL.0, true);
        }
    }

    pub unsafe fn cleanup() {
        if HOOK.0 as usize != 0 { let _ = UnhookWindowsHookEx(HOOK); }
    }

    pub unsafe fn timeout() {
        if *addr_of!(LWIN_SUPPRESSED) || *addr_of!(LSHIFT_SUPPRESSED) { Self::replay(); }
    }

    unsafe fn replay() {
        if *addr_of!(LWIN_SUPPRESSED) {
            *addr_of_mut!(LWIN_SUPPRESSED) = false;
            Self::inject_key(VK_LWIN.0, false);
        }
        if *addr_of!(LSHIFT_SUPPRESSED) {
            *addr_of_mut!(LSHIFT_SUPPRESSED) = false;
            Self::inject_key(VK_LSHIFT.0, false);
        }
        *addr_of_mut!(PRESS_STATE) = State::Idle;
        let _ = KillTimer(MAIN_HWND, 9998); 
    }

    unsafe fn inject_key(vk: u16, keyup: bool) {
        let mut input = INPUT { r#type: INPUT_KEYBOARD, ..Default::default() };
        input.Anonymous.ki.wVk = VIRTUAL_KEY(vk);
        if keyup { input.Anonymous.ki.dwFlags = KEYEVENTF_KEYUP; }
        let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

unsafe extern "system" fn config_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as u32;
            let code = (wparam.0 >> 16) as u32;
            if id == 1 && code == 0 { let _ = EnableWindow(EDIT_HWND, BOOL(0)); } 
            else if id == 2 && code == 0 { let _ = EnableWindow(EDIT_HWND, BOOL(1)); } 
            else if id == 4 && code == 0 { // Save
                let is_run = SendDlgItemMessageW(hwnd, 2, 0x00F0 /*BM_GETCHECK*/, WPARAM(0), LPARAM(0)).0 as u32 == 1;
                
                let new_type: u32;
                let mut new_data: Vec<u16>;

                if is_run {
                    new_type = 1;
                    new_data = vec![0u16; 512];
                    let len = GetWindowTextW(EDIT_HWND, &mut new_data);
                    new_data.truncate(len as usize); new_data.push(0);
                } else {
                    new_type = 0; 
                    new_data = w!("VK_RCONTROL").as_wide().to_vec();
                }

                *addr_of_mut!(ACTION_TYPE) = new_type; 
                *addr_of_mut!(ACTION_DATA) = new_data.clone();
                
                let mut hkey = windows::Win32::System::Registry::HKEY::default();
                let subkey = w!("Software\\Gallery Inc\\IBBE-Hooker\\NoCopilot");
                if windows::Win32::System::Registry::RegCreateKeyExW(windows::Win32::System::Registry::HKEY_CURRENT_USER, subkey, 0, PCWSTR::null(), windows::Win32::System::Registry::REG_OPTION_NON_VOLATILE, windows::Win32::System::Registry::KEY_ALL_ACCESS, None, &mut hkey, None).is_ok() {
                    let type_bytes = std::slice::from_raw_parts(&new_type as *const _ as *const u8, 4);
                    let _ = windows::Win32::System::Registry::RegSetValueExW(hkey, w!("ActionType"), 0, windows::Win32::System::Registry::REG_DWORD, Some(type_bytes));
                    
                    let data_bytes = std::slice::from_raw_parts(new_data.as_ptr() as *const u8, new_data.len() * 2);
                    let _ = windows::Win32::System::Registry::RegSetValueExW(hkey, w!("ActionData"), 0, windows::Win32::System::Registry::REG_SZ, Some(data_bytes));
                    
                    let _ = windows::Win32::System::Registry::RegCloseKey(hkey);
                }
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_CLOSE => { let _ = DestroyWindow(hwnd); LRESULT(0) }
        WM_DESTROY => { CONFIG_HWND = HWND(0 as _); LRESULT(0) }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 || !*addr_of_mut!(IS_ENABLED) { return CallNextHookEx(HOOK, code, wparam, lparam); }

    let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
    let vk = kb.vkCode as u16;
    let is_injected = (kb.flags.0 & LLKHF_INJECTED.0) != 0;
    let is_keyup = wparam.0 as u32 == WM_KEYUP || wparam.0 as u32 == WM_SYSKEYUP;

    if is_injected { return CallNextHookEx(HOOK, code, wparam, lparam); }

    if !is_keyup { 
        if vk == VK_LWIN.0 {
            NoCopilotManager::replay();
            *addr_of_mut!(LWIN_SUPPRESSED) = true;
            *addr_of_mut!(PRESS_STATE) = State::LWin;
            let _ = SetTimer(MAIN_HWND, 9998, 30, None);
            return LRESULT(-1);
        }
        if *addr_of!(PRESS_STATE) == State::LWin {
            if vk == VK_LSHIFT.0 {
                *addr_of_mut!(LSHIFT_SUPPRESSED) = true;
                *addr_of_mut!(PRESS_STATE) = State::LShift;
                return LRESULT(-1);
            } else { NoCopilotManager::replay(); }
        } else if *addr_of!(PRESS_STATE) == State::LShift {
            if vk == VK_F23.0 {
                *addr_of_mut!(PRESS_STATE) = State::Idle; 
                *addr_of_mut!(RELEASE_STATE) = State::F23;
                let _ = KillTimer(MAIN_HWND, 9998);
                *addr_of_mut!(LSHIFT_SUPPRESSED) = false; 
                *addr_of_mut!(LWIN_SUPPRESSED) = false;
                NoCopilotManager::execute_action();
                return LRESULT(-1);
            } else { NoCopilotManager::replay(); }
        }
    } else {
        if vk == VK_F23.0 && *addr_of!(RELEASE_STATE) == State::F23 {
            *addr_of_mut!(RELEASE_STATE) = State::LShift;
            NoCopilotManager::finish_action();
            return LRESULT(-1);
        }
        if vk == VK_LSHIFT.0 && *addr_of!(RELEASE_STATE) == State::LShift { 
            *addr_of_mut!(RELEASE_STATE) = State::LWin; return LRESULT(-1); 
        }
        if vk == VK_LWIN.0 && *addr_of!(RELEASE_STATE) == State::LWin { 
            *addr_of_mut!(RELEASE_STATE) = State::Idle; return LRESULT(-1); 
        }
        
        if *addr_of!(PRESS_STATE) != State::Idle {
            let lwin_sup = *addr_of!(LWIN_SUPPRESSED); 
            let lshift_sup = *addr_of!(LSHIFT_SUPPRESSED);
            NoCopilotManager::replay();
            if lwin_sup && vk == VK_LWIN.0 { NoCopilotManager::inject_key(VK_LWIN.0, true); return LRESULT(-1); }
            if lshift_sup && vk == VK_LSHIFT.0 { NoCopilotManager::inject_key(VK_LSHIFT.0, true); return LRESULT(-1); }
        }
    }
    CallNextHookEx(HOOK, code, wparam, lparam)
}