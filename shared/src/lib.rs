#![no_std]

pub const APP_VERSION: &str = "1.0.29565";
pub const MANAGER_WINDOW_CLASS: windows::core::PCWSTR = windows::core::w!("WindowEnhancerManagerClass");

pub const WM_ENHANCER_ACTION: u32 = 0x8001;
pub const WM_TRAY_CALLBACK: u32 = 0x8002;
pub const WM_ENHANCER_RESTORE_SNAP: u32 = 0x8003;

pub const IDM_ALWAYS_ON_TOP: u32 = 0x7A10;
pub const IDM_TRANSPARENT: u32 = 0x7A20;
pub const IDM_ROLL: u32 = 0x7A30;
pub const IDM_TRAY: u32 = 0x7A40;
pub const IDM_SEPARATOR: u32 = 0x7A50;

pub fn tr_w(lang_id: u32, key: &str) -> alloc::vec::Vec<u16> {
    let text = match key {
        "menu_snap" => match lang_id { 0 => "Functie: Window Snapping", 1 => "Feature: Window Snapping", _ => "Funkció: Ablak illesztés" },
        "menu_alttab" => match lang_id { 0 => "Functie: Alt-Tab Overlay", 1 => "Feature: Alt-Tab Overlay", _ => "Funkció: Alt-Tab Overlay" },
        "menu_copilot" => match lang_id { 0 => "Functie: Blocare Tasta Copilot", 1 => "Feature: Block Copilot Key", _ => "Funkció: Copilot gomb blokkolása" },
        "menu_config_copilot" => match lang_id { 0 => "-> Configurare Actiune Copilot", 1 => "-> Configure Copilot Action", _ => "-> Copilot művelet beállítása" },
        "menu_memclean" => match lang_id { 0 => "Functie: Curatare Auto-Memorie", 1 => "Feature: Auto-Clean Memory", _ => "Funkció: Auto-Memória Tisztítás" },
        "menu_run_memclean" => match lang_id { 0 => "-> Curata Memoria Acum", 1 => "-> Clean Memory Now", _ => "-> Memória tisztítása most" },
        "menu_fancyzones" => match lang_id { 0 => "Functie: FancyZones", 1 => "Feature: FancyZones", _ => "Funkció: FancyZones" },
        "menu_edit_fancyzones" => match lang_id { 0 => "-> Editeaza Zonele", 1 => "-> Edit Zones", _ => "-> Zónák szerkesztése" },
        "menu_startup" => match lang_id { 0 => "Pornire automata (Startup)", 1 => "Run on Startup", _ => "Automatikus indítás" },
        "menu_about" => match lang_id { 0 => "Despre IBB-Hooker", 1 => "About IBB-Hooker", _ => "Névjegy: IBB-Hooker" },
        "menu_exit" => match lang_id { 0 => "Opreste IBB-Hooker (Exit)", 1 => "Exit IBB-Hooker", _ => "Kilépés" },
        "menu_lang" => match lang_id { 0 => "Limba (Language)", 1 => "Language", _ => "Nyelv" },
        "menu_restore" => match lang_id { 0 => "Restabilire fereastra", 1 => "Restore Window", _ => "Ablak visszaállítása" },
        "menu_close" => match lang_id { 0 => "Inchide aplicatia", 1 => "Close Application", _ => "Alkalmazás bezárása" },
        "msg_already" => match lang_id { 0 => "IBB-Hooker ruleaza deja!", 1 => "IBB-Hooker is already running!", _ => "Az IBB-Hooker már fut!" },
        "msg_warn" => match lang_id { 0 => "Avertizare", 1 => "Warning", _ => "Figyelmeztetés" },
        "msg_about_app" => match lang_id { 0 => "Aplicatie administrata de IBB-Hooker.", 1 => "Application managed by IBB-Hooker.", _ => "Az IBB-Hooker által kezelt." },
        // Inside tr_w match block:
        "menu_hotkeys" => match lang_id { 
            0 => "-> Manager Hotkeys (F13-F24)", 
            1 => "-> Hotkey Manager (F13-F24)", 
            _ => "-> Gyorsbillentyű kezelő (F13-F24)" 
        },
        // Setup Copilot
        "cfg_title" => match lang_id { 0 => "Configurare buton Copilot", 1 => "Configure Copilot button", _ => "Copilot gomb beállítása" },
        "cfg_radio1" => match lang_id { 0 => "Inlocuieste cu tasta R-Control (Implicit)", 1 => "Replace with R-Control (Default)", _ => "Csere R-Control billentyűre (Alapértelmezett)" },
        "cfg_radio2" => match lang_id { 0 => "Ruleaza Executabil sau Adresa Web (URL)", 1 => "Run Executable or Web Address (URL)", _ => "Futtatható fájl vagy webcím (URL) indítása" },
        "cfg_save" => match lang_id { 0 => "Salveaza", 1 => "Save", _ => "Mentés" },
        
        // Setup FancyZones
        "fz_editor_title" => match lang_id { 0 => "Editor Zone (Deseneaza, Esc pt Salvare)", 1 => "Zone Editor (Draw, Esc to Save)", _ => "Zónaszerkesztő (Rajzolás, Esc a mentéshez)" },
        "fz_zone_label" => match lang_id { 0 => "Zona", 1 => "Zone", _ => "Zóna" },
        
        _ => "Unknown"
    };
    let mut v: alloc::vec::Vec<u16> = text.encode_utf16().collect();
    v.push(0); 
    v
}

extern crate alloc;