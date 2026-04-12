#![windows_subsystem = "windows"]

mod always_on_top; mod rollup; mod transparency; mod tray; mod menu;
mod startupreg; mod app_killer; mod explorerstarts; mod advanced_paste;
mod settings; mod nocopilot; mod mem_cleaner;

use always_on_top::{AlwaysOnTopManager, overlay_wnd_proc};
use rollup::RollUpManager; use transparency::TransparencyManager;
use tray::TrayManager; use menu::*;
use startupreg::StartupManager; use app_killer::AppKiller;
use windowmanager::DesktopManager;
use explorerstarts::ExplorerStarts;
use advanced_paste::AdvancedPasteManager;

use shared::*;
use std::ptr::addr_of_mut;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, WPARAM, RECT};
use windows::Win32::Graphics::Gdi::*; 
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const APP_TRAY_ICON_ID: u32 = 1;
const OBJID_WINDOW: i32 = 0;
const TIMER_ACTION_DELAY: usize = 9999; 
const TIMER_MEM_CLEANER: usize = 9997; 

const HOTKEY_TOP: i32 = 1;
const HOTKEY_TRANSP: i32 = 2; 
const HOTKEY_ROLL: i32 = 3;   
const HOTKEY_TRAY: i32 = 4;   
const HOTKEY_KILL: i32 = 5;   
const HOTKEY_PASTE: i32 = 6; 

unsafe fn is_fullscreen(hwnd: HWND) -> bool {
    let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
    if (style & WS_CAPTION.0) == WS_CAPTION.0 { return false; }
    let mut rect = RECT::default(); if GetWindowRect(hwnd, &mut rect).is_err() { return false; }
    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST); if monitor.0 as usize == 0 { return false; }
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    if GetMonitorInfoW(monitor, &mut mi).as_bool() {
        return rect.left <= mi.rcMonitor.left && rect.top <= mi.rcMonitor.top && rect.right >= mi.rcMonitor.right && rect.bottom >= mi.rcMonitor.bottom;
    }
    false
}

struct WindowManager { top: AlwaysOnTopManager, roll: RollUpManager, tray: TrayManager }
impl WindowManager { fn new() -> Self { Self { top: AlwaysOnTopManager::new(), roll: RollUpManager::new(), tray: TrayManager::new() } } }

static mut MANAGER: Option<WindowManager> = None;
static mut RUNDLL32_PROCESS: Option<std::process::Child> = None;
static mut PENDING_ACTIONS: Vec<(u32, HWND)> = Vec::new();
static mut SETTINGS: Option<settings::Settings> = None;

unsafe extern "system" fn cleanup_callback(hwnd: HWND, _: LPARAM) -> BOOL {
    let hmenu = GetSystemMenu(hwnd, false);
    if !hmenu.is_invalid() {
        let _ = DeleteMenu(hmenu, IDM_ALWAYS_ON_TOP, MF_BYCOMMAND); let _ = DeleteMenu(hmenu, IDM_TRANSPARENT, MF_BYCOMMAND);
        let _ = DeleteMenu(hmenu, IDM_ROLL, MF_BYCOMMAND); let _ = DeleteMenu(hmenu, IDM_TRAY, MF_BYCOMMAND); let _ = DeleteMenu(hmenu, IDM_SEPARATOR, MF_BYCOMMAND);
    }
    BOOL(1) 
}

unsafe extern "system" fn win_event_proc(_hook: HWINEVENTHOOK, event: u32, hwnd: HWND, id_object: i32, _child: i32, _thread: u32, _time: u32) {
    if event == EVENT_OBJECT_LOCATIONCHANGE && id_object == OBJID_WINDOW {
        if let Some(manager) = (*addr_of_mut!(MANAGER)).as_mut() { manager.top.sync_overlay(hwnd); }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_HOTKEY => {
            let hotkey_id = wparam.0 as i32; let target = GetForegroundWindow();
            if target.0 as usize != 0 {
                if hotkey_id == HOTKEY_KILL { AppKiller::kill_target(target); }
                else if hotkey_id == HOTKEY_PASTE { AdvancedPasteManager::show(); }
                else if !is_fullscreen(target) {
                    if let Some(manager) = (*addr_of_mut!(MANAGER)).as_mut() {
                        match hotkey_id {
                            HOTKEY_TOP => manager.top.toggle(target), HOTKEY_TRANSP => TransparencyManager::toggle(target),
                            HOTKEY_ROLL => manager.roll.toggle(target), HOTKEY_TRAY => manager.tray.send_to_tray(target, hwnd),
                            _ => {}
                        }
                    }
                }
            }
            LRESULT(0)
        }
        WM_ENHANCER_RESTORE_SNAP => { let target = HWND(lparam.0 as _); if !is_fullscreen(target) { DesktopManager::restore_if_snapped(target); } LRESULT(0) }
        WM_ENHANCER_ACTION => {
            let target = HWND(lparam.0 as _);
            if !is_fullscreen(target) {
                let action = wparam.0 as u32; (*addr_of_mut!(PENDING_ACTIONS)).push((action, target)); let _ = SetTimer(hwnd, TIMER_ACTION_DELAY, 50, None);
            }
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == 9998 {
                nocopilot::NoCopilotManager::timeout();
            } else if wparam.0 == TIMER_MEM_CLEANER { 
                if let Some(s) = (*addr_of_mut!(SETTINGS)).as_ref() {
                    if s.mem_cleaner_enabled { mem_cleaner::MemCleaner::clean(); }
                }
            } else if wparam.0 == TIMER_ACTION_DELAY {
                let _ = KillTimer(hwnd, TIMER_ACTION_DELAY); let actions: Vec<_> = (*addr_of_mut!(PENDING_ACTIONS)).drain(..).collect();
                if let Some(manager) = (*addr_of_mut!(MANAGER)).as_mut() {
                    for (action, target) in actions { MenuManager::handle_titlebar_action(action, target, hwnd, &mut manager.top, &mut manager.roll, &mut manager.tray); }
                }
            }
            LRESULT(0)
        }
        WM_TRAY_CALLBACK => {
            let icon_id = wparam.0 as u32; let mouse_msg = (lparam.0 & 0xFFFF) as u32; 

            if icon_id == APP_TRAY_ICON_ID {
                if mouse_msg == WM_RBUTTONUP || mouse_msg == WM_CONTEXTMENU {
                    let mut pt = POINT::default(); let _ = GetCursorPos(&mut pt);
                    let is_startup = StartupManager::is_registered();
                    let s = (*addr_of_mut!(SETTINGS)).as_mut().unwrap();
                    let cmd_id = MenuManager::show_main_tray_menu(hwnd, pt, is_startup, s);
                    
                    if cmd_id == CMD_MAIN_ABOUT { 
                        let v = format!("IBB-Hooker v{}\nGallery Inc.", shared::APP_VERSION); let mut u = v.encode_utf16().collect::<Vec<u16>>(); u.push(0);
                        let txt_about = shared::tr_w(s.language, "menu_about");
                        let _ = MessageBoxW(hwnd, PCWSTR(u.as_ptr()), PCWSTR(txt_about.as_ptr()), MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND); 
                    }
                    else if cmd_id == CMD_MAIN_STARTUP { StartupManager::toggle(); } 
                    else if cmd_id == CMD_MAIN_EXIT { let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0)); }
                    else if cmd_id == CMD_MAIN_TOGGLE_SNAP { s.snap_enabled = !s.snap_enabled; s.save(); DesktopManager::set_snap_enabled(s.snap_enabled); }
                    else if cmd_id == CMD_MAIN_TOGGLE_ALTTAB { s.alttab_enabled = !s.alttab_enabled; s.save(); DesktopManager::set_alttab_enabled(s.alttab_enabled); }
                    else if cmd_id == CMD_MAIN_TOGGLE_COPILOT { s.copilot_enabled = !s.copilot_enabled; s.save(); nocopilot::IS_ENABLED = s.copilot_enabled; }
                    else if cmd_id == CMD_MAIN_TOGGLE_MEMCLEAN { 
                        s.mem_cleaner_enabled = !s.mem_cleaner_enabled; s.save(); 
                        if s.mem_cleaner_enabled { mem_cleaner::MemCleaner::clean(); } 
                    }
                    else if cmd_id == CMD_MAIN_RUN_MEMCLEAN { // ACTIUNEA MANUALA: Rulăm instantaneu funcția clean
                        mem_cleaner::MemCleaner::clean();
                    }
                    else if cmd_id == CMD_MAIN_LANG_RO { s.language = 0; s.save(); }
                    else if cmd_id == CMD_MAIN_LANG_EN { s.language = 1; s.save(); }
                    else if cmd_id == CMD_MAIN_LANG_HU { s.language = 2; s.save(); }
                }
            } else {
                let tray_id = icon_id;
                if mouse_msg == WM_LBUTTONUP || mouse_msg == WM_LBUTTONDOWN { 
                    if let Some(manager) = (*addr_of_mut!(MANAGER)).as_mut() { manager.tray.restore_from_tray(tray_id, hwnd); }
                } else if mouse_msg == WM_RBUTTONUP || mouse_msg == WM_CONTEXTMENU {
                    let mut pt = POINT::default(); let _ = GetCursorPos(&mut pt);
                    let s = (*addr_of_mut!(SETTINGS)).as_ref().unwrap();
                    let cmd_id = MenuManager::show_app_tray_menu(hwnd, pt, s);
                    if let Some(manager) = (*addr_of_mut!(MANAGER)).as_mut() {
                        if cmd_id == CMD_APP_RESTORE { manager.tray.restore_from_tray(tray_id, hwnd); }
                        else if cmd_id == CMD_APP_CLOSE { manager.tray.close_from_tray(tray_id, hwnd); }
                        else if cmd_id == CMD_APP_ABOUT { 
                            let txt_msg = shared::tr_w(s.language, "msg_about_app"); let txt_about = shared::tr_w(s.language, "menu_about");
                            let _ = MessageBoxW(hwnd, PCWSTR(txt_msg.as_ptr()), PCWSTR(txt_about.as_ptr()), MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND); 
                        }
                    }
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let mut nid = NOTIFYICONDATAW::default();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd; nid.uID = APP_TRAY_ICON_ID;
            Shell_NotifyIconW(NIM_DELETE, &nid);

            let _ = EnumWindows(Some(cleanup_callback), LPARAM(0));
            nocopilot::NoCopilotManager::cleanup();

            unsafe {
                use windows::Win32::System::Threading::{OpenEventW, SetEvent, EVENT_MODIFY_STATE};
                let hmod_name = HSTRING::from("hook-x64.dll");
                if let Ok(module) = LoadLibraryW(&hmod_name) {
                    if let Some(proc) = GetProcAddress(module, windows::core::s!("UninstallHook")) {
                        let uninstall_hook: extern "system" fn() = std::mem::transmute(proc); uninstall_hook();
                    }
                }
                let event_name = w!("Global\\IBBHooker_ExitEvent");
                if let Ok(hevent) = OpenEventW(EVENT_MODIFY_STATE, BOOL(0), event_name) { let _ = SetEvent(hevent); let _ = windows::Win32::Foundation::CloseHandle(hevent); }
            }

            std::thread::sleep(std::time::Duration::from_millis(500));
            let child_ptr = addr_of_mut!(RUNDLL32_PROCESS); if let Some(mut child) = (*child_ptr).take() { let _ = child.kill(); }
            DesktopManager::cleanup_all();
            DestroyWindow(hwnd).unwrap(); LRESULT(0)
        }
        WM_DESTROY => { PostQuitMessage(0); LRESULT(0) }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("Eroare Critica in Manager:\n{}", info);
        unsafe { let _ = MessageBoxW(HWND(0 as _), &HSTRING::from(msg), w!("Crash"), MB_ICONERROR | MB_TOPMOST); }
    }));
}

fn main() {
    setup_panic_hook();
    unsafe {
        let existing_hwnd = FindWindowW(MANAGER_WINDOW_CLASS, None);
        if existing_hwnd.0 as usize != 0 {
            let loaded_settings = settings::Settings::load();
            let msg = shared::tr_w(loaded_settings.language, "msg_already"); let warn = shared::tr_w(loaded_settings.language, "msg_warn");
            let _ = MessageBoxW(HWND(0 as _), PCWSTR(msg.as_ptr()), PCWSTR(warn.as_ptr()), MB_OK | MB_ICONWARNING | MB_SETFOREGROUND);
            return; 
        }

        MANAGER = Some(WindowManager::new());
        let hinstance = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap();

        let wc_main = WNDCLASSW { lpfnWndProc: Some(wnd_proc), hInstance: hinstance.into(), lpszClassName: MANAGER_WINDOW_CLASS, ..Default::default() }; RegisterClassW(&wc_main);
        let wc_overlay = WNDCLASSW { lpfnWndProc: Some(overlay_wnd_proc), hInstance: hinstance.into(), lpszClassName: w!("WindowEnhancerOverlayClass"), ..Default::default() }; RegisterClassW(&wc_overlay);

        let hwnd = CreateWindowExW(WINDOW_EX_STYLE(0), MANAGER_WINDOW_CLASS, w!("Manager"), WINDOW_STYLE(0), 0, 0, 0, 0, HWND_MESSAGE, None, hinstance, None);
        let _ = ChangeWindowMessageFilterEx(hwnd, WM_ENHANCER_ACTION, MSGFLT_ALLOW, None);

        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd; nid.uID = APP_TRAY_ICON_ID; nid.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE;
        nid.uCallbackMessage = WM_TRAY_CALLBACK; nid.hIcon = LoadIconW(None, IDI_APPLICATION).unwrap();
        let tip = format!("IBB-Hooker v{}\0", shared::APP_VERSION).encode_utf16().collect::<Vec<u16>>();
        for (i, &c) in tip.iter().enumerate() { if i < nid.szTip.len() { nid.szTip[i] = c; } }
        Shell_NotifyIconW(NIM_ADD, &nid);

        RegisterHotKey(hwnd, HOTKEY_TOP, MOD_CONTROL | MOD_WIN, 0x79).expect("Eroare F10"); 
        RegisterHotKey(hwnd, HOTKEY_TRANSP, MOD_CONTROL | MOD_WIN, 0x78).expect("Eroare F9"); 
        RegisterHotKey(hwnd, HOTKEY_ROLL, MOD_CONTROL | MOD_WIN, 0x7A).expect("Eroare F11"); 
        RegisterHotKey(hwnd, HOTKEY_TRAY, MOD_CONTROL | MOD_WIN, 0x7B).expect("Eroare F12");
        RegisterHotKey(hwnd, HOTKEY_KILL, MOD_CONTROL | MOD_ALT, 0x73).expect("Eroare F4");

        let _hook = SetWinEventHook(EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE, None, Some(win_event_proc), 0, 0, WINEVENT_OUTOFCONTEXT);
        let _hook_explorer = ExplorerStarts::initialize();
        
        let mut dll_path = std::env::current_exe().unwrap(); dll_path.set_file_name("hook-x64.dll");
        let hmod = LoadLibraryW(&HSTRING::from(dll_path.to_str().unwrap())).expect("Lipseste hook-x64.dll!");
        let install_hook: extern "system" fn() -> bool = std::mem::transmute(GetProcAddress(hmod, windows::core::s!("InstallHook")).unwrap());
        if !install_hook() { panic!("Injectare x64 Esuata!"); }

        use std::os::windows::process::CommandExt;
        let mut dll32_path = std::env::current_exe().unwrap(); dll32_path.set_file_name("hook-x86.dll");
        if dll32_path.exists() {
            let args = format!("{},RunHook32", dll32_path.display());
            RUNDLL32_PROCESS = std::process::Command::new("C:\\Windows\\SysWOW64\\rundll32.exe").arg(args).creation_flags(0x08000000).spawn().ok();
        }

        let loaded_settings = settings::Settings::load();
        DesktopManager::set_snap_enabled(loaded_settings.snap_enabled);
        DesktopManager::set_alttab_enabled(loaded_settings.alttab_enabled);
        nocopilot::IS_ENABLED = loaded_settings.copilot_enabled;
        
        if loaded_settings.mem_cleaner_enabled { mem_cleaner::MemCleaner::clean(); }
        
        SETTINGS = Some(loaded_settings);
        let _ = SetTimer(hwnd, TIMER_MEM_CLEANER, 600_000, None);

        nocopilot::NoCopilotManager::init(hwnd);
        DesktopManager::initialize_all();
        AdvancedPasteManager::init();
        
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(0 as _), 0, 0).into() { TranslateMessage(&msg); DispatchMessageW(&msg); }
    }
}