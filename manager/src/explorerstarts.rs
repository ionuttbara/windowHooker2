use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, WPARAM};
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use std::ptr::addr_of_mut;

static mut PROCESSED_HWNDS: Vec<isize> = Vec::new();

pub struct ExplorerStarts;

impl ExplorerStarts {
    pub unsafe fn initialize() -> HWINEVENTHOOK {
        // Punem hook pentru crearea ferestrelor noi
        SetWinEventHook(
            EVENT_OBJECT_CREATE,
            EVENT_OBJECT_CREATE,
            None,
            Some(Self::win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    }

    unsafe extern "system" fn win_event_proc(
        _hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        _thread: u32,
        _time: u32,
    ) {
        // CORECTURA AICI: Extragem valoarea i32 din OBJID_WINDOW cu `.0` și convertim CHILDID_SELF `as i32`
        if event == EVENT_OBJECT_CREATE && id_object == OBJID_WINDOW.0 && id_child == CHILDID_SELF as i32 {
            if hwnd.0 as usize == 0 { return; }

            // Verificăm să nu reluăm la nesfârșit o fereastră deja procesată
            let processed = &mut *addr_of_mut!(PROCESSED_HWNDS);
            if processed.contains(&(hwnd.0 as isize)) {
                return;
            }

            // Păstrăm un buffer curat pentru a evita leak-urile de memorie
            if processed.len() > 100 {
                processed.drain(0..50);
            }

            // Obținem clasa ferestrei
            let mut class_name = [0u16; 256];
            let len = GetClassNameW(hwnd, &mut class_name);
            if len == 0 { return; }
            let name = String::from_utf16_lossy(&class_name[..len as usize]);

            // Căutăm DOAR ferestrele noi de tip Explorer (CabinetWClass)
            if name == "CabinetWClass" {
                let mut target_pid = 0;
                GetWindowThreadProcessId(hwnd, Some(&mut target_pid));

                let shell_hwnd = GetShellWindow();
                let mut shell_pid = 0;
                if shell_hwnd.0 as usize != 0 {
                    GetWindowThreadProcessId(shell_hwnd, Some(&mut shell_pid));
                }

                // Dacă este proces separat de shell-ul principal, declanșăm schema
                if target_pid != shell_pid && shell_pid != 0 {
                    processed.push(hwnd.0 as isize);
                    Self::apply_f11_fix(hwnd);
                }
            }
        }
    }

    unsafe fn apply_f11_fix(hwnd: HWND) {
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        let is_layered = (ex_style & WS_EX_LAYERED.0) != 0;

        // Imediat ce fereastra a fost creată, o facem transparentă (alpha 1 din 255)
        // Nu folosim 0, deoarece unele API-uri de Windows ignoră complet ferestrele cu alpha 0 la input
        if !is_layered {
            let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, (ex_style | WS_EX_LAYERED.0) as i32);
        }
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 1, LWA_ALPHA);

        let hwnd_val = hwnd.0 as isize;
        
        // Derulăm pe un thread separat pentru a NU bloca mesajele din manager.exe
        std::thread::spawn(move || {
            let target = HWND(hwnd_val as _);

            // 1. Așteptăm un pic să se inițializeze controalele din fereastră
            std::thread::sleep(std::time::Duration::from_millis(350));

            // Parametrii corecți care imită o tastă fizică apasată de om pentru F11
            let lparam_down = LPARAM(0x00570001); // Scan code 87
            let lparam_up = LPARAM(0xC0570001);

            // 2. Apăsăm F11 (Intrare in FullScreen)
            let _ = PostMessageW(target, WM_KEYDOWN, WPARAM(VK_F11.0 as usize), lparam_down);
            let _ = PostMessageW(target, WM_KEYUP, WPARAM(VK_F11.0 as usize), lparam_up);

            // 3. Așteptăm animația și tranziția internă a Explorer-ului (ribbon reload)
            std::thread::sleep(std::time::Duration::from_millis(150));

            // 4. Apăsăm F11 (Ieșire din FullScreen)
            let _ = PostMessageW(target, WM_KEYDOWN, WPARAM(VK_F11.0 as usize), lparam_down);
            let _ = PostMessageW(target, WM_KEYUP, WPARAM(VK_F11.0 as usize), lparam_up);

            // 5. Așteptăm ieșirea
            std::thread::sleep(std::time::Duration::from_millis(150));

            // 6. Readucem la vizibilitate completă
            let _ = SetLayeredWindowAttributes(target, COLORREF(0), 255, LWA_ALPHA);

            // Ștergem flag-ul LAYERED dacă fereastra nu îl avea inițial, pentru a evita bug-uri grafice
            if !is_layered {
                let current_ex_style = GetWindowLongW(target, GWL_EXSTYLE) as u32;
                let _ = SetWindowLongW(target, GWL_EXSTYLE, (current_ex_style & !WS_EX_LAYERED.0) as i32);
            }
        });
    }
}