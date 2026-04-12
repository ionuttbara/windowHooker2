use shared::*;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{BOOL, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, WPARAM, CloseHandle};
use windows::Win32::System::LibraryLoader::{
    GetModuleHandleExW, GetModuleFileNameW, 
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT
};
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject, INFINITE};
use windows::Win32::UI::WindowsAndMessaging::*;

static mut HOOK_HANDLE_CALLWND: HHOOK = HHOOK(0 as _);
static mut HOOK_HANDLE_GETMSG: HHOOK = HHOOK(0 as _);

// Prevenim injectarea în DWM, CSRSS și Winlogon pentru a evita glitch-urile grafice
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn DllMain(
    _hinst: HINSTANCE,
    reason: u32,
    _reserved: *mut std::ffi::c_void,
) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        let mut path = [0u16; 260]; // MAX_PATH
        let len = GetModuleFileNameW(HINSTANCE(0), &mut path);
        let exe_path = String::from_utf16_lossy(&path[..len as usize]).to_lowercase();
        
        if exe_path.ends_with("dwm.exe") || 
           exe_path.ends_with("winlogon.exe") || 
           exe_path.ends_with("csrss.exe") 
        {
            return BOOL(0);
        }
    }
    BOOL(1)
}

#[export_name = "InstallHook"] 
pub unsafe extern "system" fn install_hook() -> bool {
    let mut h_module: HMODULE = HMODULE(0 as _);
    let _ = GetModuleHandleExW(
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        PCWSTR(install_hook as *const () as *const u16),
        &mut h_module,
    );

    if h_module.0 as usize == 0 { return false; }

    let hook_call = SetWindowsHookExW(WH_CALLWNDPROC, Some(call_wnd_proc), h_module, 0);
    let hook_get = SetWindowsHookExW(WH_GETMESSAGE, Some(get_msg_proc), h_module, 0);

    if let (Ok(h1), Ok(h2)) = (hook_call, hook_get) {
        HOOK_HANDLE_CALLWND = h1;
        HOOK_HANDLE_GETMSG = h2;
        true
    } else {
        false
    }
}

#[export_name = "UninstallHook"]
pub unsafe extern "system" fn uninstall_hook() {
    if HOOK_HANDLE_CALLWND.0 as usize != 0 { 
        let _ = UnhookWindowsHookEx(HOOK_HANDLE_CALLWND); 
        HOOK_HANDLE_CALLWND = HHOOK(0 as _); 
    }
    if HOOK_HANDLE_GETMSG.0 as usize != 0 { 
        let _ = UnhookWindowsHookEx(HOOK_HANDLE_GETMSG); 
        HOOK_HANDLE_GETMSG = HHOOK(0 as _); 
    }
}

#[export_name = "RunHook32"] 
pub unsafe extern "system" fn run_hook32(_hwnd: HWND, _hinst: HINSTANCE, _cmdline: *mut i8, _show: i32) {
    if install_hook() {
        // Creăm Event-ul la care procesul x86 așteaptă în liniște
        let event_name = w!("Global\\IBBHooker_ExitEvent");
        let hevent = CreateEventW(None, BOOL(1), BOOL(0), event_name).unwrap();

        // Thread-ul de injectare rămâne blocat aici până se închide aplicația
        WaitForSingleObject(hevent, INFINITE);
        let _ = CloseHandle(hevent);

        // Dezinstalare sigură
        uninstall_hook();
    }
}

unsafe fn process_syscommand(message: u32, wparam: WPARAM, hwnd: HWND) {
    let cmd_id = if message == WM_SYSCOMMAND { (wparam.0 & 0xFFF0) as u32 } else { (wparam.0 & 0xFFFF) as u32 };

    let manager_hwnd = FindWindowW(MANAGER_WINDOW_CLASS, PCWSTR::null());
    if manager_hwnd.0 as usize != 0 {
        if cmd_id == SC_MOVE {
            let _ = SendMessageW(manager_hwnd, WM_ENHANCER_RESTORE_SNAP, WPARAM(0), LPARAM(hwnd.0 as _));
        } 
        else if cmd_id == IDM_ALWAYS_ON_TOP || cmd_id == IDM_TRANSPARENT || cmd_id == IDM_ROLL || cmd_id == IDM_TRAY {
            let _ = PostMessageW(manager_hwnd, WM_ENHANCER_ACTION, WPARAM(cmd_id as usize), LPARAM(hwnd.0 as _));
        }
    }
}

unsafe extern "system" fn get_msg_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let msg = &*(lparam.0 as *const MSG);
        if msg.message == WM_SYSCOMMAND || msg.message == WM_COMMAND {
            process_syscommand(msg.message, msg.wParam, msg.hwnd);
        }
    }
    CallNextHookEx(HOOK_HANDLE_GETMSG, code, wparam, lparam)
}

unsafe extern "system" fn call_wnd_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let details = &*(lparam.0 as *const CWPSTRUCT);

        if details.message == WM_INITMENUPOPUP {
            let style = GetWindowLongW(details.hwnd, GWL_STYLE) as u32;
            
            if (style & WS_CAPTION.0) != WS_CAPTION.0 || (style & WS_SYSMENU.0) == 0 {
                return CallNextHookEx(HOOK_HANDLE_CALLWND, code, wparam, lparam);
            }

            let hmenu = HMENU(details.wParam.0 as _);
            let is_sys_menu = (details.lParam.0 >> 16) & 0xFFFF != 0;

            if !hmenu.is_invalid() && is_sys_menu {
                if GetMenuState(hmenu, IDM_ALWAYS_ON_TOP, MF_BYCOMMAND) == u32::MAX {
                    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
                    let _ = AppendMenuW(hmenu, MF_STRING, IDM_ALWAYS_ON_TOP as usize, w!("Always on Top"));
                    let _ = AppendMenuW(hmenu, MF_STRING, IDM_TRANSPARENT as usize, w!("Transparent Window"));
                    let _ = AppendMenuW(hmenu, MF_STRING, IDM_ROLL as usize, w!("Roll up / Down"));
                    let _ = AppendMenuW(hmenu, MF_STRING, IDM_TRAY as usize, w!("Send to Tray"));
                }

                let ex_style = GetWindowLongW(details.hwnd, GWL_EXSTYLE) as u32;
                let is_topmost = (ex_style & WS_EX_TOPMOST.0) != 0;
                let state_top = if is_topmost { MF_CHECKED } else { MF_UNCHECKED };
                let _ = CheckMenuItem(hmenu, IDM_ALWAYS_ON_TOP, MF_BYCOMMAND.0 | state_top.0);
                
                let is_transparent = (ex_style & WS_EX_LAYERED.0) != 0;
                let state_transp = if is_transparent { MF_CHECKED } else { MF_UNCHECKED };
                let _ = CheckMenuItem(hmenu, IDM_TRANSPARENT, MF_BYCOMMAND.0 | state_transp.0);
            }
        } else if details.message == WM_SYSCOMMAND || details.message == WM_COMMAND {
            process_syscommand(details.message, details.wParam, details.hwnd);
        }
    }
    
    CallNextHookEx(HOOK_HANDLE_CALLWND, code, wparam, lparam)
}