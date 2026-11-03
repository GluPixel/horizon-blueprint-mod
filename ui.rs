//! UI system for blueprint mod - screens, HUD, and menus

use std::sync::{Arc, Mutex};
use horizon_client::mod_api::{Host, ScreenDrawEvent, Text};
use horizon_client::mod_api::NativeScreen;
use crate::blueprint::Blueprint;
use crate::config::Config;
use crate::material_list::MaterialList;

/// Blueprint UI handler
pub struct BlueprintUi {
    config: Arc<Mutex<Config>>,
    show_file_browser: bool,
    file_browser_path: std::path::PathBuf,
    selected_file_index: usize,
    show_material_list: bool,
    show_settings: bool,
    hud_visible: bool,
}

impl BlueprintUi {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        let file_browser_path = dirs::home_dir()
            .map(|p| p.join("schematics"))
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        Self {
            config,
            show_file_browser: false,
            file_browser_path,
            selected_file_index: 0,
            show_material_list: false,
            show_settings: false,
            hud_visible: true,
        }
    }

    /// Draw HUD overlay
    pub fn draw_hud(&self, _host: &dyn Host, _event: &mut ScreenDrawEvent<'_>) {
        let config = self.config.lock().unwrap();
        if !config.ui.show_hud {
            return;
        }
        drop(config);

        // TODO: Draw HUD using screen drawing API
        // For now, placeholder
    }

    /// Create native screen for file browser
    pub fn create_file_browser_screen(&self) -> NativeScreen {
        NativeScreen {
            width: 300.0,
            height: 400.0,
            slots: vec![],
            inventory_y: 0.0,
            inventory_x: 0.0,
            own_labels: true,
            own_background: true,
        }
    }

    /// Create native screen for material list
    pub fn create_material_list_screen(&self, _material_list: &MaterialList) -> NativeScreen {
        NativeScreen {
            width: 400.0,
            height: 500.0,
            slots: vec![],
            inventory_y: 0.0,
            inventory_x: 0.0,
            own_labels: true,
            own_background: true,
        }
    }

    /// Create native screen for settings
    pub fn create_settings_screen(&self) -> NativeScreen {
        NativeScreen {
            width: 350.0,
            height: 450.0,
            slots: vec![],
            inventory_y: 0.0,
            inventory_x: 0.0,
            own_labels: true,
            own_background: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_creation() {
        let config = Arc::new(Mutex::new(Config::default()));
        let ui = BlueprintUi::new(config);
        assert!(!ui.show_file_browser);
        assert!(ui.hud_visible);
    }
}