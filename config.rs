//! Configuration for the Blueprint mod

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Hologram settings
    pub hologram: HologramConfig,
    /// Printer settings
    pub printer: PrinterConfig,
    /// UI settings
    pub ui: UiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hologram: HologramConfig::default(),
            printer: PrinterConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HologramConfig {
    /// Whether the hologram is enabled
    pub enabled: bool,
    /// Alpha/transparency of the hologram (0.0 - 1.0)
    pub alpha: f32,
    /// Render mode: "wireframe" or "model"
    pub render_mode: String,
    /// Highlight missing blocks in red
    pub highlight_missing: bool,
    /// Highlight incorrectly placed blocks in yellow
    pub highlight_incorrect: bool,
    /// Show block outlines only
    pub outlines_only: bool,
    /// Maximum blocks to render (for performance)
    pub max_blocks: usize,
}

impl Default for HologramConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            alpha: 0.5,
            render_mode: "wireframe".to_string(), // "wireframe" or "model"
            highlight_missing: true,
            highlight_incorrect: true,
            outlines_only: false,
            max_blocks: 50000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfig {
    /// Whether printer mode is enabled
    pub enabled: bool,
    /// Maximum range from player to place blocks (in blocks)
    pub range: f32,
    /// Blocks per second placement speed
    pub speed: f32,
    /// Only work in survival mode
    pub survival_only: bool,
    /// Skip air blocks (don't place air)
    pub skip_air: bool,
    /// Require block support (don't place floating blocks)
    pub require_support: bool,
    /// Delay between placements in milliseconds
    pub delay_ms: u64,
    /// Only use items from main inventory (not ender chest, shulker boxes)
    pub main_inventory_only: bool,
}

impl Default for PrinterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            range: 6.0,
            speed: 20.0,
            survival_only: true,
            skip_air: true,
            require_support: true,
            delay_ms: 50,
            main_inventory_only: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Show HUD overlay with blueprint info
    pub show_hud: bool,
    /// HUD position: "top_left", "top_right", "bottom_left", "bottom_right"
    pub hud_position: String,
    /// Show material list in HUD
    pub show_material_list: bool,
    /// Keybinding for toggling hologram
    pub key_toggle_hologram: String,
    /// Keybinding for moving up
    pub key_move_up: String,
    /// Keybinding for moving down
    pub key_move_down: String,
    /// Keybinding for rotating
    pub key_rotate: String,
    /// Keybinding for mirroring
    pub key_mirror: String,
    /// Keybinding for toggling printer
    pub key_toggle_printer: String,
    /// Keybinding for material list
    pub key_material_list: String,
    /// Keybinding for loading blueprint
    pub key_load_blueprint: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_hud: true,
            hud_position: "top_left".to_string(),
            show_material_list: true,
            key_toggle_hologram: "key.keyboard.b".to_string(),
            key_move_up: "key.keyboard.space".to_string(),
            key_move_down: "key.keyboard.left.control".to_string(),
            key_rotate: "key.keyboard.r".to_string(),
            key_mirror: "key.keyboard.m".to_string(),
            key_toggle_printer: "key.keyboard.p".to_string(),
            key_material_list: "key.keyboard.l".to_string(),
            key_load_blueprint: "key.keyboard.l".to_string(),
        }
    }
}

impl Config {
    /// Load config from file
    pub fn load(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    /// Save config to file
    pub fn save(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}