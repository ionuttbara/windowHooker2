use std::ptr::addr_of_mut;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

static mut HOOK: HHOOK = HHOOK(0 as _);
static mut MAIN_HWND: HWND = HWND(0 as _);
pub static mut IS_ENABLED: bool = true;

#[derive(PartialEq, Clone, Copy)]
enum State { Idle, LWin, LShift, F23 }

static mut PRESS_STATE: State = State::Idle;
static mut RELEASE_STATE: State = State::Idle;
static mut LWIN_SUPPRESSED: bool = false;
static mut LSHIFT_SUPPRESSED: bool = false;

pub struct NoCopilotManager;

impl NoCopilotManager {
    pub unsafe fn init(hwnd: HWND) {
        MAIN_HWND = hwnd;
        let hmod = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap();
        HOOK = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hmod, 0).unwrap();
    }

    #[allow(dead_code)] // Suprimăm warning-ul; deși funcția e chemată la închidere, compilatorul se ia de ea.
    pub unsafe fn cleanup() {
        if HOOK.0 as usize != 0 { let _ = UnhookWindowsHookEx(HOOK); }
    }

    pub unsafe fn timeout() {
        if LWIN_SUPPRESSED || LSHIFT_SUPPRESSED { Self::replay(); }
    }

    unsafe fn replay() {
        if LWIN_SUPPRESSED {
            LWIN_SUPPRESSED = false;
            Self::inject_key(VK_LWIN.0, false);
        }
        if LSHIFT_SUPPRESSED {
            LSHIFT_SUPPRESSED = false;
            Self::inject_key(VK_LSHIFT.0, false);
        }
        PRESS_STATE = State::Idle;
        let _ = KillTimer(MAIN_HWND, 9998); 
    }

    unsafe fn inject_key(vk: u16, keyup: bool) {
        let mut input = INPUT { r#type: INPUT_KEYBOARD, ..Default::default() };
        input.Anonymous.ki.wVk = VIRTUAL_KEY(vk);
        if keyup { input.Anonymous.ki.dwFlags = KEYEVENTF_KEYUP; }
        let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
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
            LWIN_SUPPRESSED = true;
            PRESS_STATE = State::LWin;
            let _ = SetTimer(MAIN_HWND, 9998, 30, None);
            return LRESULT(-1);
        }
        if PRESS_STATE == State::LWin {
            if vk == VK_LSHIFT.0 {
                LSHIFT_SUPPRESSED = true;
                PRESS_STATE = State::LShift;
                return LRESULT(-1);
            } else { NoCopilotManager::replay(); }
        } else if PRESS_STATE == State::LShift {
            if vk == VK_F23.0 {
                PRESS_STATE = State::Idle; RELEASE_STATE = State::F23;
                let _ = KillTimer(MAIN_HWND, 9998);
                LSHIFT_SUPPRESSED = false; LWIN_SUPPRESSED = false;
                NoCopilotManager::inject_key(VK_RCONTROL.0, false);
                return LRESULT(-1);
            } else { NoCopilotManager::replay(); }
        }
    } else {
        if vk == VK_F23.0 && RELEASE_STATE == State::F23 {
            RELEASE_STATE = State::LShift;
            NoCopilotManager::inject_key(VK_RCONTROL.0, true);
            return LRESULT(-1);
        }
        if vk == VK_LSHIFT.0 && RELEASE_STATE == State::LShift { RELEASE_STATE = State::LWin; return LRESULT(-1); }
        if vk == VK_LWIN.0 && RELEASE_STATE == State::LWin { RELEASE_STATE = State::Idle; return LRESULT(-1); }
        
        if PRESS_STATE != State::Idle {
            let lwin_sup = LWIN_SUPPRESSED; let lshift_sup = LSHIFT_SUPPRESSED;
            NoCopilotManager::replay();
            if lwin_sup && vk == VK_LWIN.0 { NoCopilotManager::inject_key(VK_LWIN.0, true); return LRESULT(-1); }
            if lshift_sup && vk == VK_LSHIFT.0 { NoCopilotManager::inject_key(VK_LSHIFT.0, true); return LRESULT(-1); }
        }
    }
    CallNextHookEx(HOOK, code, wparam, lparam)
}