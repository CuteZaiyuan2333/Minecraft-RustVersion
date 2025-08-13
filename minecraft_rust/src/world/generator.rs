use bevy::prelude::*;
use noise::{NoiseFn, Perlin, Seedable};
use crate::world::chunk::{Chunk, BlockId};
use crate::block_registry::BlockRegistry;

/// 世界生成器配置
#[derive(Resource, Clone)]
pub struct WorldGeneratorConfig {
    pub seed: u32,
    pub sea_level: i32,
    pub max_height: i32,
    pub min_height: i32,
    pub terrain_scale: f64,
    pub terrain_octaves: usize,
    pub cave_threshold: f64,
    pub ore_frequency: f64,
}

impl Default for WorldGeneratorConfig {
    fn default() -> Self {
        Self {
            seed: 12345,
            sea_level: 64,
            max_height: 128,
            min_height: 0,
            terrain_scale: 0.01,
            terrain_octaves: 4,
            cave_threshold: 0.6,
            ore_frequency: 0.02,
        }
    }
}

/// 世界生成器
pub struct WorldGenerator {
    terrain_noise: Perlin,
    cave_noise: Perlin,
    ore_noise: Perlin,
    config: WorldGeneratorConfig,
}

impl WorldGenerator {
    pub fn new(config: WorldGeneratorConfig) -> Self {
        let mut terrain_noise = Perlin::new(config.seed);
        terrain_noise = terrain_noise.set_seed(config.seed);
        
        let mut cave_noise = Perlin::new(config.seed + 1);
        cave_noise = cave_noise.set_seed(config.seed + 1);
        
        let mut ore_noise = Perlin::new(config.seed + 2);
        ore_noise = ore_noise.set_seed(config.seed + 2);

        Self {
            terrain_noise,
            cave_noise,
            ore_noise,
            config,
        }
    }

    /// 生成区块地形
    pub fn generate_chunk(&self, chunk: &mut Chunk, registry: &BlockRegistry) {
        let chunk_world_x = chunk.coord.x * 32;
        let chunk_world_z = chunk.coord.z * 32;
        let chunk_world_y = chunk.coord.y * 32;

        for x in 0..32 {
            for z in 0..32 {
                let world_x = chunk_world_x + x as i32;
                let world_z = chunk_world_z + z as i32;
                
                // 生成地形高度
                let height = self.generate_height(world_x, world_z);
                
                for y in 0..32 {
                    let world_y = chunk_world_y + y as i32;
                    
                    let block_id = if world_y <= height {
                        self.generate_block_at(world_x, world_y, world_z, height, registry)
                    } else {
                        BlockId::Air
                    };
                    
                    chunk.set_block(x as u32, y as u32, z as u32, block_id);
                }
            }
        }
    }

    /// 获取指定位置的地面高度（公共方法）
    pub fn get_surface_height(&self, x: i32, z: i32) -> i32 {
        self.generate_height(x, z)
    }

    /// 生成指定位置的地形高度
    fn generate_height(&self, x: i32, z: i32) -> i32 {
        let mut height = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = self.config.terrain_scale;

        // 多层噪声叠加
        for _ in 0..self.config.terrain_octaves {
            height += self.terrain_noise.get([x as f64 * frequency, z as f64 * frequency]) * amplitude;
            amplitude *= 0.5;
            frequency *= 2.0;
        }

        // 将噪声值映射到高度范围
        let normalized_height = (height + 1.0) * 0.5; // 将 [-1,1] 映射到 [0,1]
        let terrain_height = self.config.min_height as f64 + 
            normalized_height * (self.config.max_height - self.config.min_height) as f64;

        terrain_height as i32
    }

    /// 生成指定位置的方块类型
    fn generate_block_at(&self, x: i32, y: i32, z: i32, surface_height: i32, registry: &BlockRegistry) -> BlockId {
        // 检查是否是洞穴
        if self.is_cave(x, y, z) {
            return BlockId::Air;
        }

        // 基岩层
        if y <= self.config.min_height + 2 {
            return registry.get_block_id("bedrock").unwrap_or(BlockId::Bedrock);
        }

        // 地表层
        if y == surface_height {
            if surface_height > self.config.sea_level {
                // 高于海平面的地表是草方块
                return registry.get_block_id("grass").unwrap_or(BlockId::Grass);
            } else {
                // 海平面及以下的地表是泥土
                return registry.get_block_id("dirt").unwrap_or(BlockId::Dirt);
            }
        }

        // 地下层
        if y > surface_height - 4 && y < surface_height {
            // 表层下的泥土
            return registry.get_block_id("dirt").unwrap_or(BlockId::Dirt);
        }

        // 检查矿物生成
        if let Some(ore_block) = self.generate_ore(x, y, z, registry) {
            return ore_block;
        }

        // 默认石头
        registry.get_block_id("stone").unwrap_or(BlockId::Stone)
    }

    /// 检查指定位置是否是洞穴
    fn is_cave(&self, x: i32, y: i32, z: i32) -> bool {
        // 避免在地表附近生成洞穴
        if y > self.config.sea_level + 10 {
            return false;
        }

        let cave_value = self.cave_noise.get([
            x as f64 * 0.02,
            y as f64 * 0.02,
            z as f64 * 0.02,
        ]);

        cave_value > self.config.cave_threshold
    }

    /// 生成矿物
    fn generate_ore(&self, x: i32, y: i32, z: i32, _registry: &BlockRegistry) -> Option<BlockId> {
        let ore_value = self.ore_noise.get([
            x as f64 * 0.05,
            y as f64 * 0.05,
            z as f64 * 0.05,
        ]);

        if ore_value > 0.7 {
            // 根据深度生成不同的矿物
            if y < 16 {
                // 深层：钻石矿（暂时用基岩代替）
                Some(BlockId::Bedrock)
            } else if y < 32 {
                // 中层：铁矿（暂时用石头代替）
                Some(BlockId::Stone)
            } else {
                // 浅层：煤矿（暂时用石头代替）
                Some(BlockId::Stone)
            }
        } else {
            None
        }
    }
}

/// 生物群系类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiomeType {
    Plains,
    Forest,
    Desert,
    Mountains,
    Ocean,
}

impl BiomeType {
    /// 根据温度和湿度确定生物群系
    pub fn from_climate(temperature: f64, humidity: f64) -> Self {
        match (temperature, humidity) {
            (t, h) if t > 0.8 && h < 0.3 => BiomeType::Desert,
            (t, h) if t > 0.6 && h > 0.6 => BiomeType::Forest,
            (t, _h) if t < 0.2 => BiomeType::Mountains,
            (t, _) if t < 0.0 => BiomeType::Ocean,
            _ => BiomeType::Plains,
        }
    }

    /// 获取生物群系的地表方块
    pub fn surface_block(&self, registry: &BlockRegistry) -> BlockId {
        match self {
            BiomeType::Plains | BiomeType::Forest => {
                registry.get_block_id("grass").unwrap_or(BlockId::Grass)
            }
            BiomeType::Desert => {
                registry.get_block_id("dirt").unwrap_or(BlockId::Dirt) // 沙子，暂时用泥土
            }
            BiomeType::Mountains => {
                registry.get_block_id("stone").unwrap_or(BlockId::Stone)
            }
            BiomeType::Ocean => {
                registry.get_block_id("dirt").unwrap_or(BlockId::Dirt)
            }
        }
    }
}