use bunny_plugin::{
    PluginContext, PluginInfo, bunny_ui::ui::BunnyUi, hook_builder::NoCbHookPoint,
    hook_cell::HookCell,
};
use tracing::error;

use crate::{address::Addresses, hooks::on_quest_update, ui::State};

const PLUGIN_NAME: &str = "Damage Numbers";
const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");

pub static STATE: HookCell<State> = HookCell::new();
static HOOKS: HookCell<Vec<NoCbHookPoint>> = HookCell::new();

// Called once when the plugin is loaded
#[unsafe(no_mangle)]
pub extern "C" fn init(context: &PluginContext) -> PluginInfo {
    tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(context.log_level())
        .init();

    let addresses = Addresses::new(context.mhfo_info());
    let state = State::new(context);
    let state_res = STATE.set(state);
    let mut info = PluginInfo::new(PLUGIN_NAME, PLUGIN_VERSION)
        .ui_menu(menu)
        .ui_free(ui)
        .save(save)
        .quest_hook(on_quest_update);

    match crate::hooks::init(&addresses) {
        Ok(hooks) => {
            let hooks_res = HOOKS.set(hooks);
            if state_res.is_err() || hooks_res.is_err() {
                info = info.init_fail("State/Hooks init failed: HookCell was already initialized");
            }
            info
        }
        Err(e) => {
            let err_message = format!("Hook init error: {e:#}");
            info = info.init_fail(err_message.as_str());
            error!(err_message);
            info
        }
    }
}

// Called every frame when the plugin's dropdown in the manager window is open
pub extern "C" fn menu(ui: &mut BunnyUi) {
    let state = unsafe { STATE.get_unchecked_mut() };
    state.menu(ui);
}

// Called every frame
pub extern "C" fn ui(ui: &mut BunnyUi) {
    let state = unsafe { STATE.get_unchecked_mut() };
    state.ui(ui);
}

// Called once per user defined autosave interval, and when the plugin is manually disabled by the user or the game is closed
pub extern "C" fn save() {
    if let Some(state) = STATE.get() {
        state.save_config();
    }
}

// Called when the plugin is manually disabled by the user
pub fn unload() {
    unsafe {
        HOOKS.drop();
        STATE.drop();
    }
}
