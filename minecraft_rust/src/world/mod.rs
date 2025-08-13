use bevy::prelude::*;
use self::chunk::{Chunk, BlockId};
use self::storage::ChunkStorage;
use self::generator::{WorldGenerator, WorldGeneratorConfig};
use crate::block_registry::BlockRegistry;
use crate::game_state::GameState;

pub mod chunk;
pub mod storage;
pub mod generator;
pub mod chunk_loader;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkStorage::new())
           .insert_resource(WorldGeneratorConfig::default())
           .add_plugins(chunk_loader::ChunkLoaderPlugin)
           .add_systems(OnEnter(GameState::InGame), setup_world);
    }
}

#[derive(Resource, Clone)]
pub struct WorldConfig {
    pub chunk_size: UVec3, // 32x32x32
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self { chunk_size: UVec3::new(32, 32, 32) }
    }
}

#[derive(Resource, Default)]
pub struct WorldState {
    pub chunks_loaded: usize,
}

fn setup_world(
    mut commands: Commands, 
    chunk_storage: Res<ChunkStorage>,
    registry: Res<BlockRegistry>,
    generator_config: Res<WorldGeneratorConfig>
) {
    // 创建世界生成器
    let generator = WorldGenerator::new(generator_config.clone());
    
    // Generate a 5x5 area of chunks for better terrain visibility
    for x in -2..=2 {
        for z in -2..=2 {
            for y in 0..=2 { // 生成多层区块以展示地形高度变化
                let chunk_pos = IVec3::new(x, y, z);
                let chunk_world_pos = Vec3::new(
                    x as f32 * 32.0,
                    y as f32 * 32.0,
                    z as f32 * 32.0,
                );

                // Create chunk entity
                let mut chunk = Chunk::new(chunk_pos);
                
                // Generate terrain for this chunk using the new generator
                generator.generate_chunk(&mut chunk, &registry);
                chunk.compute_solid_blocks();
                
                // Spawn chunk entity
                let chunk_entity = commands
                    .spawn((
                        chunk,
                        SpatialBundle {
                            transform: Transform::from_translation(chunk_world_pos),
                            ..default()
                        },
                    ))
                    .id();

                // Store chunk in storage
                chunk_storage.insert(chunk_pos, chunk_entity);
            }
        }
    }
}