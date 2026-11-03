//! Material list calculation and display

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use horizon_client::mod_api::Host;
use horizon_client::items::ItemStack;
use horizon_world::registry::{BlockRegistry, BlockStateId};
use crate::blueprint::{Blueprint, BlockInfo};
use crate::config::Config;

/// Material entry for a block type
#[derive(Debug, Clone)]
pub struct MaterialEntry {
    pub block_state: BlockStateId,
    pub block_name: String,
    pub required: usize,
    pub available: usize,
    pub missing: usize,
    pub in_main_inventory: usize,
    pub in_offhand: usize,
    pub in_ender_chest: usize,
    pub in_shulker_boxes: usize,
}

/// Material list for a blueprint
pub struct MaterialList {
    config: Arc<Mutex<Config>>,
    entries: Vec<MaterialEntry>,
    total_required: usize,
    total_available: usize,
    total_missing: usize,
    last_update: std::time::Instant,
    needs_refresh: bool,
}

impl MaterialList {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        Self {
            config,
            entries: Vec::new(),
            total_required: 0,
            total_available: 0,
            total_missing: 0,
            last_update: std::time::Instant::now(),
            needs_refresh: true,
        }
    }

    /// Update material list from blueprint and player inventory
    pub fn update(&mut self, host: &mut dyn Host, blueprint: Option<&Blueprint>, registry: &BlockRegistry) {
        let now = std::time::Instant::now();
        if !self.needs_refresh && now.duration_since(self.last_update).as_millis() < 500 {
            return; // Throttle updates
        }

        self.entries.clear();
        self.total_required = 0;
        self.total_available = 0;
        self.total_missing = 0;

        let blueprint = match blueprint {
            Some(bp) => bp,
            None => {
                self.needs_refresh = false;
                self.last_update = now;
                return;
            }
        };

        // Count required blocks
        let required_counts = blueprint.material_list();

        // Get player inventory
        let inventory_counts = self.get_inventory_counts(host, registry);

        // Build entries
        for (block_state, required) in required_counts {
            let block_name = registry.state(block_state)
                .map(|s| s.name.as_str().to_string())
                .unwrap_or_else(|| format!("unknown:{}", block_state.0));

            let available = inventory_counts.get(&block_state).copied().unwrap_or(0);
            let missing = required.saturating_sub(available);

            self.total_required += required;
            self.total_available += available;
            self.total_missing += missing;

            // Get detailed inventory breakdown
            let (main, offhand, ender, shulker) = self.get_inventory_breakdown(host, block_state);

            self.entries.push(MaterialEntry {
                block_state,
                block_name,
                required,
                available,
                missing,
                in_main_inventory: main,
                in_offhand: offhand,
                in_ender_chest: ender,
                in_shulker_boxes: shulker,
            });
        }

        // Sort by missing count (most missing first), then by name
        self.entries.sort_by(|a, b| {
            b.missing.cmp(&a.missing)
                .then_with(|| a.block_name.cmp(&b.block_name))
        });

        self.needs_refresh = false;
        self.last_update = now;
    }

    /// Get total counts across all inventory
    fn get_inventory_counts(&self, host: &mut dyn Host, registry: &BlockRegistry) -> HashMap<BlockStateId, usize> {
        let mut counts = HashMap::new();

        // This would iterate through player inventory
        // For now, return empty - real implementation would access inventory
        counts
    }

    /// Get detailed inventory breakdown
    fn get_inventory_breakdown(&self, host: &mut dyn Host, block_state: BlockStateId) -> (usize, usize, usize, usize) {
        // Real implementation would check:
        // - Main inventory (slots 9-35)
        // - Hotbar (slots 0-8)
        // - Offhand (slot 40)
        // - Ender chest
        // - Shulker boxes in inventory
        (0, 0, 0, 0)
    }

    /// Force refresh on next update
    pub fn mark_dirty(&mut self) {
        self.needs_refresh = true;
    }

    /// Get entries for display
    pub fn entries(&self) -> &[MaterialEntry] {
        &self.entries
    }

    /// Get summary totals
    pub fn totals(&self) -> (usize, usize, usize) {
        (self.total_required, self.total_available, self.total_missing)
    }

    /// Get completion percentage
    pub fn completion_percentage(&self) -> f32 {
        if self.total_required == 0 {
            100.0
        } else {
            (self.total_available as f32 / self.total_required as f32) * 100.0
        }
    }

    /// Export as text
    pub fn export_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("Material List: {} required, {} available, {} missing ({:.1}%)\n",
            self.total_required, self.total_available, self.total_missing, self.completion_percentage()));
        out.push_str(&format!("{}\n", "-".repeat(60)));

        for entry in &self.entries {
            let status = if entry.missing == 0 { "✓" } else { "✗" };
            out.push_str(&format!("{} {}: {}/{} (missing: {})\n",
                status, entry.block_name, entry.available, entry.required, entry.missing));
        }

        out
    }

    /// Export as JSON
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        #[derive(serde::Serialize)]
        struct ExportEntry {
            block_name: String,
            required: usize,
            available: usize,
            missing: usize,
        }

        #[derive(serde::Serialize)]
        struct ExportData {
            total_required: usize,
            total_available: usize,
            total_missing: usize,
            completion_percentage: f32,
            entries: Vec<ExportEntry>,
        }

        let data = ExportData {
            total_required: self.total_required,
            total_available: self.total_available,
            total_missing: self.total_missing,
            completion_percentage: self.completion_percentage(),
            entries: self.entries.iter().map(|e| ExportEntry {
                block_name: e.block_name.clone(),
                required: e.required,
                available: e.available,
                missing: e.missing,
            }).collect(),
        };

        serde_json::to_string_pretty(&data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_list_creation() {
        let config = Arc::new(Mutex::new(Config::default()));
        let list = MaterialList::new(config);
        assert_eq!(list.totals(), (0, 0, 0));
        assert_eq!(list.completion_percentage(), 100.0);
    }
}