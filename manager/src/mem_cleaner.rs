use windows::core::{w, s};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA, GetModuleHandleA};

pub struct MemCleaner;

impl MemCleaner {
    pub unsafe fn clean() {
        type OptFn = unsafe extern "system" fn(isize, u32, *mut isize) -> i32;
        type LpvFn = unsafe extern "system" fn(*const u16, *const u16, *mut [u32; 2]) -> i32;
        type AtpFn = unsafe extern "system" fn(isize, i32, *const std::ffi::c_void, u32, *mut std::ffi::c_void, *mut u32) -> i32;
        type SsfcFn = unsafe extern "system" fn(usize, usize, u32) -> i32;

        let advapi32 = LoadLibraryA(s!("advapi32.dll")).unwrap_or_default();
        let kernel32 = GetModuleHandleA(s!("kernel32.dll")).unwrap_or_default();

        if advapi32.0 as usize != 0 && kernel32.0 as usize != 0 {
            let opt: Option<OptFn> = std::mem::transmute(GetProcAddress(advapi32, s!("OpenProcessToken")));
            let lpv: Option<LpvFn> = std::mem::transmute(GetProcAddress(advapi32, s!("LookupPrivilegeValueW")));
            let atp: Option<AtpFn> = std::mem::transmute(GetProcAddress(advapi32, s!("AdjustTokenPrivileges")));
            let ssfc: Option<SsfcFn> = std::mem::transmute(GetProcAddress(kernel32, s!("SetSystemFileCacheSize")));

            if let (Some(opt), Some(lpv), Some(atp), Some(ssfc)) = (opt, lpv, atp, ssfc) {
                let mut h_token = 0isize;
                // TOKEN_ADJUST_PRIVILEGES = 0x0020
                if opt(-1, 0x0020, &mut h_token) != 0 { 
                    let mut luid = [0u32; 2];
                    if lpv(std::ptr::null(), w!("SeIncreaseQuotaPrivilege").as_ptr(), &mut luid) != 0 {
                        #[repr(C)]
                        struct Tp { count: u32, luid: [u32; 2], attributes: u32 }
                        let tp = Tp { count: 1, luid, attributes: 0x00000002 }; // SE_PRIVILEGE_ENABLED
                        atp(h_token, 0, &tp as *const _ as _, 0, std::ptr::null_mut(), std::ptr::null_mut());
                    }
                    let close: Option<unsafe extern "system" fn(isize)->i32> = std::mem::transmute(GetProcAddress(kernel32, s!("CloseHandle")));
                    if let Some(c) = close { c(h_token); }
                }
                
                // Flush System File Cache (Golește Memoria Standby)
                ssfc(usize::MAX, usize::MAX, 0);
            }
        }
    }
}