use bevy::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BlockId {
    Air,
    Stone,
    Dirt,
    Grass,
    Bedrock,
}

impl Default for BlockId { fn default() -> Self { BlockId::Air } }

#[derive(Component, Serialize, Deserialize, Clone)]
pub struct Chunk {
    pub coord: IVec3,
    #[serde(with = "serde_bytes")]
    pub blocks: Vec<u8>,
    pub solid_blocks: Vec<IVec3>,
    #[serde(skip)]
    pub dirty: bool,
}

impl Chunk {

    pub const SIZE: UVec3 = UVec3::new(32, 32, 32);
    pub const COUNT: usize = (32*32*32) as usize;

    pub fn new(coord: IVec3) -> Self {
        Self { coord, blocks: vec![BlockId::Air as u8; Self::COUNT], solid_blocks: Vec::new(), dirty: true }
    }

    pub fn compute_solid_blocks(&mut self) {
        self.solid_blocks.clear();
        for x in 0..Self::SIZE.x {
            for y in 0..Self::SIZE.y {
                for z in 0..Self::SIZE.z {
                    if self.get_block(x, y, z) != BlockId::Air {
                        self.solid_blocks.push(IVec3::new(x as i32, y as i32, z as i32));
                    }
                }
            }
        }
    }

    pub fn get_solid_blocks(&self) -> &[IVec3] {
        &self.solid_blocks
    }

    #[inline]
    fn index(x: u32, y: u32, z: u32) -> usize {
        // x fastest, then z, then y: (y*32 + z)*32 + x
        ((y as usize) * 32 + (z as usize)) * 32 + (x as usize)
    }

    pub fn set_block(&mut self, x: u32, y: u32, z: u32, id: BlockId) {
        let idx = Self::index(x, y, z);
        let old_block = self.blocks[idx];
        self.blocks[idx] = id as u8;
        
        // 如果方块发生了变化，标记为dirty
        if old_block != id as u8 {
            self.dirty = true;
        }
    }

    pub fn get_block(&self, x: u32, y: u32, z: u32) -> BlockId {
        let idx = Self::index(x, y, z);
        match self.blocks[idx] { 0 => BlockId::Air, 1 => BlockId::Stone, 2 => BlockId::Dirt, 3 => BlockId::Grass, 4 => BlockId::Bedrock, _ => BlockId::Air }
    }
}