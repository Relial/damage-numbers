use windows::Win32::{
    Foundation::HINSTANCE,
    System::{
        LibraryLoader::DisableThreadLibraryCalls,
        SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    },
};

use crate::plugin::{save, unload};

mod address;
mod animation;
mod config;
mod damage;
mod hooks;
mod plugin;
mod ui;

#[unsafe(no_mangle)]
extern "system" fn DllMain(hinst: HINSTANCE, fdw_reason: u32, lpv_reserved: *mut ()) -> bool {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            let _ = unsafe { DisableThreadLibraryCalls(hinst.into()) };
        }
        DLL_PROCESS_DETACH => {
            save();
            if lpv_reserved.is_null() {
                // Unloading through FreeLibrary
                unload();
            }
        }
        _ => {}
    }
    true
}
