//! Schematic file parsers for .schematic, .schem, and .litematic formats

use std::io::{Cursor, Read};
use std::path::Path;
use flate2::read::GzDecoder;
use horizon_world::nbt::{Tag, Compound, JavaString, NamedTag, read_named};
use horizon_world::registry::{BlockRegistry, BlockStateId};
use crate::blueprint::{BlockInfo, EntityInfo};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SchematicError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("NBT parsing error: {0}")]
    Nbt(String),
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

pub type Result<T> = std::result::Result<T, SchematicError>;

/// Represents a parsed schematic in our internal format
#[derive(Debug, Clone)]
pub struct Schematic {
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub length: i32,
    pub blocks: Vec<BlockInfo>,
    pub block_names: Vec<String>, // Original block names for registry lookup
    pub entities: Vec<EntityInfo>,
    pub tile_entities: Vec<Tag>,
    pub biomes: Option<Vec<i32>>,
    pub format: SchematicFormat,
    pub metadata: SchematicMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchematicFormat {
    Schematic,  // .schematic (MCEdit)
    Schem,      // .schem (gzipped MCEdit)
    Litematic,  // .litematic (Litematica)
}

#[derive(Debug, Clone, Default)]
pub struct SchematicMetadata {
    pub author: Option<String>,
    pub description: Option<String>,
    pub created_by: Option<String>,
    pub version: Option<String>,
    pub regions: Vec<RegionInfo>,
}

#[derive(Debug, Clone)]
pub struct RegionInfo {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub width: i32,
    pub height: i32,
    pub length: i32,
}

/// Parse a schematic file from a path
pub fn parse_schematic_file(path: &Path) -> Result<Schematic> {
    let extension = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let data = std::fs::read(path)?;

    match extension.as_str() {
        "schematic" => parse_schematic(&data),
        "schem" => parse_schem(&data),
        "litematic" => parse_litematic(&data),
        _ => Err(SchematicError::UnsupportedFormat(format!(
            "Unknown file extension: .{}", extension
        ))),
    }
}

/// Parse .schematic format (MCEdit NBT)
pub fn parse_schematic(data: &[u8]) -> Result<Schematic> {
    let (named_tag, _) = read_named(data, horizon_world::nbt::Limits::default())
        .map_err(|e| SchematicError::Nbt(e.to_string()))?;

    let root = match named_tag.tag {
        Tag::Compound(c) => c,
        _ => return Err(SchematicError::InvalidFormat("Root tag must be compound".to_string())),
    };

    parse_mcedit_schematic(&root, SchematicFormat::Schematic)
}

/// Parse .schem format (gzipped MCEdit)
pub fn parse_schem(data: &[u8]) -> Result<Schematic> {
    let mut decoder = GzDecoder::new(Cursor::new(data));
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    parse_schematic(&decompressed)
}

/// Parse .litematic format (Litematica)
pub fn parse_litematic(data: &[u8]) -> Result<Schematic> {
    let mut decoder = GzDecoder::new(Cursor::new(data));
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    let (named_tag, _) = read_named(&decompressed, horizon_world::nbt::Limits::default())
        .map_err(|e| SchematicError::Nbt(e.to_string()))?;

    let root = match named_tag.tag {
        Tag::Compound(c) => c,
        _ => return Err(SchematicError::InvalidFormat("Root tag must be compound".to_string())),
    };

    parse_litematic_nbt(&root)
}

/// Parse MCEdit schematic format (shared by .schematic and .schem)
fn parse_mcedit_schematic(root: &Compound, format: SchematicFormat) -> Result<Schematic> {
    let get_int = |key: &str| -> Result<i32> {
        root.get(&JavaString::from(key))
            .and_then(|t| match t {
                Tag::Short(v) => Some(*v as i32),
                Tag::Int(v) => Some(*v),
                _ => None,
            })
            .ok_or_else(|| SchematicError::MissingField(key.to_string()))
    };

    let width = get_int("Width")?;
    let height = get_int("Height")?;
    let length = get_int("Length")?;

    // Get blocks array
    let blocks_data = root.get(&JavaString::from("Blocks"))
        .ok_or_else(|| SchematicError::MissingField("Blocks".to_string()))?;
    let blocks_bytes = match blocks_data {
        Tag::ByteArray(b) => b,
        _ => return Err(SchematicError::InvalidFormat("Blocks must be ByteArray".to_string())),
    };

    // Get data array (block metadata)
    let data_data = root.get(&JavaString::from("Data"));
    let data_bytes = match data_data {
        Some(Tag::ByteArray(b)) => Some(b.clone()),
        None => None,
        _ => return Err(SchematicError::InvalidFormat("Data must be ByteArray".to_string())),
    };

    // Get palette (block IDs)
    let palette = root.get(&JavaString::from("Palette"));
    let block_palette = parse_palette(palette)?;

    // Get tile entities
    let tile_entities = root.get(&JavaString::from("TileEntities"))
        .and_then(|t| match t {
            Tag::List(list) => Some(list.clone()),
            _ => None,
        })
        .unwrap_or_default();

    // Get entities
    let entities = root.get(&JavaString::from("Entities"))
        .and_then(|t| match t {
            Tag::List(list) => Some(list.clone()),
            _ => None,
        })
        .unwrap_or_default();

    // Get biomes (optional)
    let biomes = root.get(&JavaString::from("Biomes"))
        .and_then(|t| match t {
            Tag::IntArray(arr) => Some(arr.iter().map(|v| *v as i32).collect()),
            _ => None,
        });

    // Parse metadata
    let metadata = parse_metadata(&root);

    // Convert blocks to our format
    let mut blocks = Vec::new();
    let mut block_names = Vec::new();
    let total_blocks = (width * height * length) as usize;

    for i in 0..total_blocks.min(blocks_bytes.len()) {
        let block_id = blocks_bytes[i] as u16;
        let data_val = data_bytes.as_ref().map(|b| b[i]).unwrap_or(0);

        if block_id == 0 {
            continue; // Air block
        }

        let (x, y, z) = index_to_coords(i, width, length);
        let block_state = block_palette.get(block_id as usize)
            .cloned()
            .unwrap_or_else(|| format!("minecraft:air"));

        blocks.push(BlockInfo {
            pos: [x, y, z],
            state: BlockStateId(0), // Will be resolved later with registry; 0 = air
            nbt: None,
        });
        block_names.push(block_state);
    }

    // Convert tile entities
    let mut tile_entity_infos = Vec::new();
    for te in tile_entities {
        if let Tag::Compound(c) = te {
            tile_entity_infos.push(Tag::Compound(c.clone()));
        }
    }

    // Convert entities
    let mut entity_infos = Vec::new();
    for e in entities {
        if let Tag::Compound(c) = e {
            entity_infos.push(EntityInfo::from_nbt(&c));
        }
    }

    Ok(Schematic {
        name: root.get(&JavaString::from("Name"))
            .and_then(|t| match t {
                Tag::String(s) => Some(s.to_string_lossy()),
                _ => None,
            })
            .unwrap_or_else(|| "Unnamed".to_string()),
        width,
        height,
        length,
        blocks,
        block_names,
        entities: entity_infos,
        tile_entities: tile_entity_infos,
        biomes,
        format,
        metadata,
    })
}

/// Parse Litematic format
fn parse_litematic_nbt(root: &Compound) -> Result<Schematic> {
    // Litematic has a "Regions" compound with multiple regions
    let regions = root.get(&JavaString::from("Regions"))
        .and_then(|t| match t {
            Tag::Compound(c) => Some(c),
            _ => None,
        })
        .ok_or_else(|| SchematicError::MissingField("Regions".to_string()))?;

    // For now, merge all regions into one schematic
    // TODO: Support multiple regions properly
    let mut all_blocks = Vec::new();
    let mut all_block_names = Vec::new();
    let mut all_entities = Vec::new();
    let mut all_tile_entities = Vec::new();
    let mut region_infos = Vec::new();

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut min_z = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    let mut max_z = i32::MIN;

    for (region_name, region_tag) in regions {
        if let Tag::Compound(region) = region_tag {
            let region_info = parse_litematic_region(region_name.to_string_lossy(), region, &mut all_blocks, &mut all_block_names, &mut all_entities, &mut all_tile_entities)?;

            // Extract values before moving
            let rx = region_info.x;
            let ry = region_info.y;
            let rz = region_info.z;
            let rw = region_info.width;
            let rh = region_info.height;
            let rl = region_info.length;

            region_infos.push(region_info);

            min_x = min_x.min(rx);
            min_y = min_y.min(ry);
            min_z = min_z.min(rz);
            max_x = max_x.max(rx + rw);
            max_y = max_y.max(ry + rh);
            max_z = max_z.max(rz + rl);
        }
    }

    let metadata = SchematicMetadata {
        regions: region_infos,
        ..Default::default()
    };

    Ok(Schematic {
        name: root.get(&JavaString::from("Name"))
            .and_then(|t| match t {
                Tag::String(s) => Some(s.to_string_lossy()),
                _ => None,
            })
            .unwrap_or_else(|| "Litematic".to_string()),
        width: max_x - min_x,
        height: max_y - min_y,
        length: max_z - min_z,
        blocks: all_blocks,
        block_names: all_block_names,
        entities: all_entities,
        tile_entities: all_tile_entities,
        biomes: None,
        format: SchematicFormat::Litematic,
        metadata,
    })
}

/// Parse a single Litematic region
fn parse_litematic_region(
    name: String,
    region: &Compound,
    all_blocks: &mut Vec<BlockInfo>,
    all_block_names: &mut Vec<String>,
    all_entities: &mut Vec<EntityInfo>,
    all_tile_entities: &mut Vec<Tag>,
) -> Result<RegionInfo> {
    let get_int = |key: &str| -> Result<i32> {
        region.get(&JavaString::from(key))
            .and_then(|t| match t {
                Tag::Int(v) => Some(*v),
                Tag::Short(v) => Some(*v as i32),
                _ => None,
            })
            .ok_or_else(|| SchematicError::MissingField(format!("{}.{}", name, key)))
    };

    let x = get_int("PositionX")?;
    let y = get_int("PositionY")?;
    let z = get_int("PositionZ")?;
    let width = get_int("Width")?;
    let height = get_int("Height")?;
    let length = get_int("Length")?;

    // Get block data (palette + indices)
    let block_data = region.get(&JavaString::from("BlockData"))
        .ok_or_else(|| SchematicError::MissingField("BlockData".to_string()))?;

    let palette = parse_litematic_palette(block_data)?;
    let indices = parse_litematic_indices(block_data)?;

    // Convert indices to blocks
    for (i, &palette_idx) in indices.iter().enumerate() {
        if palette_idx == 0 {
            continue; // Air
        }

        let (rx, ry, rz) = index_to_coords(i, width, length);
        let wx = x + rx;
        let wy = y + ry;
        let wz = z + rz;

        if let Some(block_state) = palette.get(palette_idx as usize) {
            all_blocks.push(BlockInfo {
                pos: [wx, wy, wz],
                state: BlockStateId(0), // Will be resolved later; 0 = air
                nbt: None,
            });
            all_block_names.push(block_state.clone());
        }
    }

    // Get tile entities
    if let Some(Tag::List(tes)) = region.get(&JavaString::from("TileEntities")) {
        for te in tes {
            if let Tag::Compound(c) = te {
                all_tile_entities.push(Tag::Compound(c.clone()));
            }
        }
    }

    // Get entities
    if let Some(Tag::List(ents)) = region.get(&JavaString::from("Entities")) {
        for e in ents {
            if let Tag::Compound(c) = e {
                all_entities.push(EntityInfo::from_nbt(c));
            }
        }
    }

    Ok(RegionInfo {
        name,
        x,
        y,
        z,
        width,
        height,
        length,
    })
}

/// Parse palette from MCEdit schematic
fn parse_palette(palette_tag: Option<&Tag>) -> Result<Vec<String>> {
    let mut palette = vec!["minecraft:air".to_string(); 256]; // Default 256 entries

    if let Some(Tag::Compound(palette_compound)) = palette_tag {
        for (key, value) in palette_compound {
            if let Tag::Int(idx) = value {
                let idx = *idx as usize;
                if idx < palette.len() {
                    palette[idx] = key.to_string_lossy().to_string();
                }
            }
        }
    }

    Ok(palette)
}

/// Parse palette from Litematic
fn parse_litematic_palette(block_data: &Tag) -> Result<Vec<String>> {
    let compound = match block_data {
        Tag::Compound(c) => c,
        _ => return Err(SchematicError::InvalidFormat("BlockData must be compound".to_string())),
    };

    let palette_tag = compound.get(&JavaString::from("Palette"))
        .ok_or_else(|| SchematicError::MissingField("Palette".to_string()))?;

    let mut palette = vec!["minecraft:air".to_string()];

    if let Tag::List(list) = palette_tag {
        for entry in list {
            if let Tag::String(s) = entry {
                palette.push(s.to_string_lossy().to_string());
            }
        }
    }

    Ok(palette)
}

/// Parse block indices from Litematic
fn parse_litematic_indices(block_data: &Tag) -> Result<Vec<u32>> {
    let compound = match block_data {
        Tag::Compound(c) => c,
        _ => return Err(SchematicError::InvalidFormat("BlockData must be compound".to_string())),
    };

    let data_tag = compound.get(&JavaString::from("Data"))
        .ok_or_else(|| SchematicError::MissingField("Data".to_string()))?;

    let mut indices = Vec::new();

    if let Tag::IntArray(arr) = data_tag {
        for &v in arr {
            indices.push(v as u32);
        }
    } else if let Tag::LongArray(arr) = data_tag {
        // Litematic uses LongArray for packed indices (4 bits per block for palette index)
        // This is a simplified version - real implementation needs proper unpacking
        for &v in arr {
            indices.push(v as u32);
        }
    }

    Ok(indices)
}

/// Convert linear index to 3D coordinates
fn index_to_coords(index: usize, width: i32, length: i32) -> (i32, i32, i32) {
    let width = width as usize;
    let length = length as usize;
    let y = index / (width * length);
    let x = (index % (width * length)) % width;
    let z = (index % (width * length)) / width;
    (x as i32, y as i32, z as i32)
}

/// Parse metadata from schematic root
fn parse_metadata(root: &Compound) -> SchematicMetadata {
    let get_string = |key: &str| -> Option<String> {
        root.get(&JavaString::from(key))
            .and_then(|t| match t {
                Tag::String(s) => Some(s.to_string_lossy()),
                _ => None,
            })
    };

    SchematicMetadata {
        author: get_string("Author"),
        description: get_string("Description"),
        created_by: get_string("CreatedBy"),
        version: get_string("Version"),
        regions: Vec::new(),
    }
}

/// Convert a Schematic to a Blueprint
pub fn schematic_to_blueprint(
    schematic: Schematic,
    registry: &BlockRegistry,
) -> crate::blueprint::Blueprint {
    use crate::blueprint::{Blueprint, BlockInfo as BlueprintBlockInfo};

    let mut blueprint_blocks = Vec::new();

    for (block, name) in schematic.blocks.into_iter().zip(schematic.block_names) {
        let state_id = registry.default_state(&name)
            .or_else(|| registry.default_state("minecraft:air"))
            .unwrap_or_else(|| BlockStateId(0)); // air

        blueprint_blocks.push(BlueprintBlockInfo {
            pos: block.pos,
            state: state_id,
            nbt: block.nbt,
        });
    }

    Blueprint {
        name: schematic.name,
        size: [schematic.width, schematic.height, schematic.length],
        blocks: blueprint_blocks,
        entities: schematic.entities,
        origin: [0, 0, 0],
        rotation: crate::blueprint::Rotation::None,
        mirror: crate::blueprint::Mirror::None,
        metadata: schematic.metadata.regions.iter()
            .map(|r| (r.name.clone(), format!("{} {} {} {} {} {}", r.x, r.y, r.z, r.width, r.height, r.length)))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_to_coords() {
        assert_eq!(index_to_coords(0, 10, 10), (0, 0, 0));
        assert_eq!(index_to_coords(5, 10, 10), (5, 0, 0));
        assert_eq!(index_to_coords(10, 10, 10), (0, 0, 1));
        assert_eq!(index_to_coords(100, 10, 10), (0, 1, 0));
        assert_eq!(index_to_coords(105, 10, 10), (5, 1, 0));
    }
}