use windows::core::{PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::always_on_top::AlwaysOnTopManager;
use crate::rollup::RollUpManager;
use crate::tray::TrayManager;
use shared::*;

pub const CMD_MAIN_STARTUP: u32 = 4000;
pub const CMD_MAIN_ABOUT: u32 = 4001;
pub const CMD_MAIN_EXIT: u32 = 4002;
pub const CMD_MAIN_TOGGLE_SNAP: u32 = 4003;
pub const CMD_MAIN_TOGGLE_ALTTAB: u32 = 4004;
pub const CMD_MAIN_TOGGLE_COPILOT: u32 = 4005;
pub const CMD_MAIN_CONFIG_COPILOT: u32 = 4008;
pub const CMD_MAIN_TOGGLE_MEMCLEAN: u32 = 4006;
pub const CMD_MAIN_RUN_MEMCLEAN: u32 = 4007;
pub const CMD_MAIN_HOTKEY_MANAGER: u32 = 4030;

pub const CMD_MAIN_TOGGLE_FANCYZONES: u32 = 4020; // <-- Adăugat
pub const CMD_MAIN_EDIT_FANCYZONES: u32 = 4021;   // <-- Adăugat

pub const CMD_MAIN_LANG_RO: u32 = 4010;
pub const CMD_MAIN_LANG_EN: u32 = 4011;
pub const CMD_MAIN_LANG_HU: u32 = 4012;

pub const CMD_APP_RESTORE: u32 = 3001;
pub const CMD_APP_CLOSE: u32 = 3002;
pub const CMD_APP_ABOUT: u32 = 3003;

pub struct MenuManager;

impl MenuManager {
    pub unsafe fn handle_titlebar_action(
        action: u32,
        target: HWND,
        _manager_hwnd: HWND,
        _top: &mut AlwaysOnTopManager,
        _roll: &mut RollUpManager,
        _tray: &mut TrayManager,
    ) {
        let _ = SetForegroundWindow(target);
        unsafe fn sim_keys(vk: u16) {
            use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP, VIRTUAL_KEY};
            let mut inputs: [INPUT; 6] = std::mem::zeroed();
            for i in 0..6 { inputs[i].r#type = INPUT_KEYBOARD; }
            inputs[0].Anonymous.ki.wVk = VIRTUAL_KEY(0x5B); 
            inputs[1].Anonymous.ki.wVk = VIRTUAL_KEY(0xA2); 
            inputs[2].Anonymous.ki.wVk = VIRTUAL_KEY(vk);   
            inputs[3].Anonymous.ki.wVk = VIRTUAL_KEY(vk); inputs[3].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
            inputs[4].Anonymous.ki.wVk = VIRTUAL_KEY(0xA2); inputs[4].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
            inputs[5].Anonymous.ki.wVk = VIRTUAL_KEY(0x5B); inputs[5].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
            let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }

        match action {
            IDM_ALWAYS_ON_TOP => sim_keys(0x79), 
            IDM_TRANSPARENT => sim_keys(0x78),   
            IDM_ROLL => sim_keys(0x7A),          
            IDM_TRAY => sim_keys(0x7B),          
            _ => {}
        }
    }

    pub unsafe fn show_main_tray_menu(hwnd: HWND, pt: POINT, is_startup: bool, settings: &crate::settings::Settings) -> u32 {
        let hmenu = CreatePopupMenu().unwrap();
        let lang = settings.language;
        
        let txt_snap = tr_w(lang, "menu_snap"); let _ = AppendMenuW(hmenu, MF_STRING, CMD_MAIN_TOGGLE_SNAP as usize, PCWSTR(txt_snap.as_ptr()));
        if settings.snap_enabled { let _ = CheckMenuItem(hmenu, CMD_MAIN_TOGGLE_SNAP, MF_BYCOMMAND.0 | MF_CHECKED.0); }

        let txt_alt = tr_w(lang, "menu_alttab"); let _ = AppendMenuW(hmenu, MF_STRING, CMD_MAIN_TOGGLE_ALTTAB as usize, PCWSTR(txt_alt.as_ptr()));
        if settings.alttab_enabled { let _ = CheckMenuItem(hmenu, CMD_MAIN_TOGGLE_ALTTAB, MF_BYCOMMAND.0 | MF_CHECKED.0); }

        let txt_cop = tr_w(lang, "menu_copilot"); let _ = AppendMenuW(hmenu, MF_STRING, CMD_MAIN_TOGGLE_COPILOT as usize, PCWSTR(txt_cop.as_ptr()));
        if settings.copilot_enabled { let _ = CheckMenuItem(hmenu, CMD_MAIN_TOGGLE_COPILOT, MF_BYCOMMAND.0 | MF_CHECKED.0); }
        
        let txt_config_cop = tr_w(lang, "menu_config_copilot"); 
        let _ = AppendMenuW(hmenu, MF_STRING, CMD_MAIN_CONFIG_COPILOT as usize, PCWSTR(txt_config_cop.as_ptr()));
        
        let txt_mem = tr_w(lang, "menu_memclean"); let _ = AppendMenuW(hmenu, MF_STRING, CMD_MAIN_TOGGLE_MEMCLEAN as usize, PCWSTR(txt_mem.as_ptr()));
        if settings.mem_cleaner_enabled { let _ = CheckMenuItem(hmenu, CMD_MAIN_TOGGLE_MEMCLEAN, MF_BYCOMMAND.0 | MF_CHECKED.0); }
        
        let txt_run_mem = tr_w(lang, "menu_run_memclean"); let _ = AppendMenuW(hmenu, MF_STRING, CMD_MAIN_RUN_MEMCLEAN as usize, PCWSTR(txt_run_mem.as_ptr()));
        
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);

        let txt_hk = tr_w(lang, "menu_hotkeys"); // Add "menu_hotkeys" to shared/lib.rs translations
        let _ = AppendMenuW(hmenu, MF_STRING, CMD_MAIN_HOTKEY_MANAGER as usize, PCWSTR(txt_hk.as_ptr()));

        // --- Meniu FancyZones ---
        let txt_fz = tr_w(lang, "menu_fancyzones"); let _ = AppendMenuW(hmenu, MF_STRING, CMD_MAIN_TOGGLE_FANCYZONES as usize, PCWSTR(txt_fz.as_ptr()));
        if settings.fancyzones_enabled { let _ = CheckMenuItem(hmenu, CMD_MAIN_TOGGLE_FANCYZONES, MF_BYCOMMAND.0 | MF_CHECKED.0); }
        
        let txt_edit_fz = tr_w(lang, "menu_edit_fancyzones"); let _ = AppendMenuW(hmenu, MF_STRING, CMD_MAIN_EDIT_FANCYZONES as usize, PCWSTR(txt_edit_fz.as_ptr()));
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);

        // --- Meniu Limbi ---
        let hlang = CreatePopupMenu().unwrap();
        let ro = "Română\0".encode_utf16().collect::<Vec<u16>>(); let _ = AppendMenuW(hlang, MF_STRING, CMD_MAIN_LANG_RO as usize, PCWSTR(ro.as_ptr()));
        let en = "English\0".encode_utf16().collect::<Vec<u16>>(); let _ = AppendMenuW(hlang, MF_STRING, CMD_MAIN_LANG_EN as usize, PCWSTR(en.as_ptr()));
        let hu = "Magyar\0".encode_utf16().collect::<Vec<u16>>(); let _ = AppendMenuW(hlang, MF_STRING, CMD_MAIN_LANG_HU as usize, PCWSTR(hu.as_ptr()));
        let checked_lang = match lang { 0 => CMD_MAIN_LANG_RO, 1 => CMD_MAIN_LANG_EN, _ => CMD_MAIN_LANG_HU };
        let _ = CheckMenuItem(hlang, checked_lang, MF_BYCOMMAND.0 | MF_CHECKED.0);
        
        let txt_lang = tr_w(lang, "menu_lang");
        let _ = AppendMenuW(hmenu, MF_POPUP, hlang.0 as usize, PCWSTR(txt_lang.as_ptr()));
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);

        let txt_start = tr_w(lang, "menu_startup"); let _ = AppendMenuW(hmenu, MF_STRING, CMD_MAIN_STARTUP as usize, PCWSTR(txt_start.as_ptr()));
        if is_startup { let _ = CheckMenuItem(hmenu, CMD_MAIN_STARTUP, MF_BYCOMMAND.0 | MF_CHECKED.0); }
        
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
        let txt_about = tr_w(lang, "menu_about"); let _ = AppendMenuW(hmenu, MF_STRING, CMD_MAIN_ABOUT as usize, PCWSTR(txt_about.as_ptr()));
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
        let txt_exit = tr_w(lang, "menu_exit"); let _ = AppendMenuW(hmenu, MF_STRING, CMD_MAIN_EXIT as usize, PCWSTR(txt_exit.as_ptr()));

        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(hmenu, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, None);
        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0)); 
        let _ = DestroyMenu(hmenu);
        cmd.0 as u32
    }

    pub unsafe fn show_app_tray_menu(hwnd: HWND, pt: POINT, settings: &crate::settings::Settings) -> u32 {
        let hmenu = CreatePopupMenu().unwrap();
        let lang = settings.language;
        
        let txt_res = tr_w(lang, "menu_restore"); let _ = AppendMenuW(hmenu, MF_STRING, CMD_APP_RESTORE as usize, PCWSTR(txt_res.as_ptr()));
        let txt_close = tr_w(lang, "menu_close"); let _ = AppendMenuW(hmenu, MF_STRING, CMD_APP_CLOSE as usize, PCWSTR(txt_close.as_ptr()));
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
        let txt_about = tr_w(lang, "msg_about_app"); let _ = AppendMenuW(hmenu, MF_STRING, CMD_APP_ABOUT as usize, PCWSTR(txt_about.as_ptr()));

        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(hmenu, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, None);
        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(hmenu);
        cmd.0 as u32
    }
}