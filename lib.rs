//! Blueprint & Automatic Building System for Horizon
//! A Litematica-style mod for importing, displaying, and building schematics.

use horizon_client::mod_api::{Hooks, Host, Flow, RenderHooks};
use std::sync::{Arc, Mutex};

pub mod blueprint;
pub mod config;
pub mod hologram;
pub mod input;
pub mod material_list;
pub mod printer;
pub mod schematic;
pub mod ui;

use blueprint::Blueprint;
use config::Config;
use hologram::BlueprintHologram;
use printer::Printer;
use ui::BlueprintUi;

/// Wrapper that implements RenderHooks by read-locking the hologram
struct RenderHooksWrapper(Arc<Mutex<BlueprintHologram>>);

impl RenderHooks for RenderHooksWrapper {
    fn render_needs(&self, event: &mut horizon_client::mod_api::RenderNeedsEvent<'_>) {
        let hologram = self.0.lock().unwrap();
        hologram.render_needs(event);
    }

    fn world_drawn(&self, event: &mut horizon_client::mod_api::WorldDrawnEvent<'_, '_>) {
        let hologram = self.0.lock().unwrap();
        hologram.world_drawn(event);
    }
}

/// Main mod struct implementing the Hooks trait
pub struct BlueprintMod {
    config: Arc<Mutex<Config>>,
    hologram: Arc<Mutex<BlueprintHologram>>,
    hologram_wrapper: RenderHooksWrapper,
    printer: Arc<Mutex<Printer>>,
    ui: Arc<Mutex<BlueprintUi>>,
    current_blueprint: Arc<Mutex<Option<Blueprint>>>,
}

impl Default for BlueprintMod {
    fn default() -> Self {
        let config = Arc::new(Mutex::new(Config::default()));
        let hologram = Arc::new(Mutex::new(BlueprintHologram::new(config.clone())));
        Self {
            hologram_wrapper: RenderHooksWrapper(hologram.clone()),
            printer: Arc::new(Mutex::new(Printer::new(config.clone()))),
            ui: Arc::new(Mutex::new(BlueprintUi::new(config.clone()))),
            current_blueprint: Arc::new(Mutex::new(None)),
            config,
            hologram,
        }
    }
}

impl Hooks for BlueprintMod {
    fn render(&self) -> Option<&dyn RenderHooks> {
        Some(&self.hologram_wrapper)
    }

    fn input_key(&self, host: &mut dyn Host, event: &mut horizon_client::mod_api::InputKeyEvent<'_>) {
        let mut ui = self.ui.lock().unwrap();
        let mut hologram = self.hologram.lock().unwrap();
        let mut printer = self.printer.lock().unwrap();
        let mut blueprint = self.current_blueprint.lock().unwrap();

        input::handle_key_event(
            host,
            event,
            &mut ui,
            &mut hologram,
            &mut printer,
            &mut *blueprint
        )
    }

    fn screen_draw(&self, host: &dyn Host, event: &mut horizon_client::mod_api::ScreenDrawEvent<'_>) {
        let ui = self.ui.lock().unwrap();
        ui.draw_hud(host, event);
    }

    fn component_types(&self, event: &mut horizon_client::mod_api::ComponentTypesEvent<'_>) {
        event.types.push("horizon:blueprint".to_string());
    }
}

/// Entry point called by the launcher
pub fn hooks() -> Box<dyn Hooks> {
    Box::new(BlueprintMod::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_creation() {
        let _mod = BlueprintMod::default();
    }
}