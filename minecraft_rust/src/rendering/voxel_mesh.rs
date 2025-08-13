use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use crate::world::chunk::{Chunk, BlockId};

const CHUNK_SIZE: u32 = 32;

#[derive(Component)]
pub struct ChunkMesh {
    pub coord: IVec3,
}

#[derive(Default)]
pub struct VoxelMeshBuilder {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl VoxelMeshBuilder {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn add_cube_face(&mut self, position: Vec3, face: CubeFace, _texture_index: usize, flip_uv: bool, vertical_flip: bool) {
        let base_index = self.positions.len() as u32;
        let normal = face.normal();

        let face_positions = match face {
            CubeFace::Top => [
                position + Vec3::new(0.0, 1.0, 0.0),
                position + Vec3::new(1.0, 1.0, 0.0),
                position + Vec3::new(1.0, 1.0, 1.0),
                position + Vec3::new(0.0, 1.0, 1.0),
            ],
            CubeFace::Bottom => [
                position + Vec3::new(0.0, 0.0, 1.0),
                position + Vec3::new(1.0, 0.0, 1.0),
                position + Vec3::new(1.0, 0.0, 0.0),
                position + Vec3::new(0.0, 0.0, 0.0),
            ],
            CubeFace::North => [
                position + Vec3::new(1.0, 0.0, 0.0),
                position + Vec3::new(0.0, 0.0, 0.0),
                position + Vec3::new(0.0, 1.0, 0.0),
                position + Vec3::new(1.0, 1.0, 0.0),
            ],
            CubeFace::South => [
                position + Vec3::new(0.0, 0.0, 1.0),
                position + Vec3::new(1.0, 0.0, 1.0),
                position + Vec3::new(1.0, 1.0, 1.0),
                position + Vec3::new(0.0, 1.0, 1.0),
            ],
            CubeFace::East => [
                position + Vec3::new(1.0, 0.0, 1.0),
                position + Vec3::new(1.0, 0.0, 0.0),
                position + Vec3::new(1.0, 1.0, 0.0),
                position + Vec3::new(1.0, 1.0, 1.0),
            ],
            CubeFace::West => [
                position + Vec3::new(0.0, 0.0, 0.0),
                position + Vec3::new(0.0, 0.0, 1.0),
                position + Vec3::new(0.0, 1.0, 1.0),
                position + Vec3::new(0.0, 1.0, 0.0),
            ],
        };
    
        let mut face_uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    
        if vertical_flip {
            for uv in face_uvs.iter_mut() {
                uv[1] = 1.0 - uv[1];
            }
        }
        if flip_uv {
            for uv in face_uvs.iter_mut() {
                uv[0] = 1.0 - uv[0];
            }
        }
    
        for (i, pos) in face_positions.iter().enumerate() {
            self.positions.push(*pos);
            self.normals.push(normal);
            self.uvs.push(face_uvs[i]);
        }
    
        let indices = if matches!(face, CubeFace::Top | CubeFace::Bottom) {
            [0, 3, 2, 0, 2, 1]
        } else {
            [0, 1, 2, 0, 2, 3]
        };
        for &index in &indices {
            self.indices.push(base_index + index);
        }
    }

    pub fn build(self) -> Mesh {
        // 兼容Bevy 0.12 API
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        
        // 转换顶点位置为数组格式
        let positions: Vec<[f32; 3]> = self.positions.iter().map(|v| [v.x, v.y, v.z]).collect();
        let normals: Vec<[f32; 3]> = self.normals.iter().map(|v| [v.x, v.y, v.z]).collect();
        
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.set_indices(Some(Indices::U32(self.indices)));
        
        mesh
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CubeFace {
    Top,
    Bottom,
    North,
    South,
    East,
    West,
}

impl CubeFace {
    pub fn normal(&self) -> Vec3 {
        match self {
            CubeFace::Top => Vec3::Y,
            CubeFace::Bottom => Vec3::NEG_Y,
            CubeFace::North => Vec3::NEG_Z,
            CubeFace::South => Vec3::Z,
            CubeFace::East => Vec3::X,
            CubeFace::West => Vec3::NEG_X,
        }
    }
}

pub fn build_chunk_mesh(chunk: &Chunk, get_neighbor: impl Fn(IVec3) -> Option<Chunk>) -> Mesh {
    let mut builder = VoxelMeshBuilder::new();
    
    // 遍历chunk中的每个方块
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let block = chunk.get_block(x, y, z);
                if block == BlockId::Air {
                    continue;
                }

                let position = Vec3::new(x as f32, y as f32, z as f32);
                
                // 检查每个面是否需要渲染 (面剔除)
                let faces_to_render = get_visible_faces(chunk, x, y, z, chunk.coord, &get_neighbor);
                
                let texture_index = get_texture_index_for_block(block);
                
                for face in faces_to_render {
                    builder.add_cube_face(position, face, texture_index, false, false);
                }
            }
        }
    }
    
    builder.build()
}

pub fn build_chunk_mesh_for_block_type(chunk: &Chunk, block_type: BlockId, get_neighbor: impl Fn(IVec3) -> Option<Chunk>) -> Mesh {
    let mut builder = VoxelMeshBuilder::new();
    
    // 只遍历指定类型的方块
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let block = chunk.get_block(x, y, z);
                if block != block_type {
                    continue;
                }

                let position = Vec3::new(x as f32, y as f32, z as f32);
                
                // 检查每个面是否需要渲染 (面剔除)
                let faces_to_render = get_visible_faces(chunk, x, y, z, chunk.coord, &get_neighbor);
                
                for face in faces_to_render {
                    builder.add_cube_face(position, face, 0, false, false); // texture_index 现在不重要了
                }
            }
        }
    }
    
    builder.build()
}

// 为草方块构建特殊的多纹理网格
pub fn build_chunk_mesh_for_grass_block(
    chunk: &Chunk,
    chunk_position: IVec3,
    _block_textures: &crate::rendering::texture_loader::BlockTextures,
    get_neighbor: impl Fn(IVec3) -> Option<Chunk>
) -> (Option<Mesh>, Option<Mesh>, Option<Mesh>) {
    let mut top_builder = VoxelMeshBuilder::new();
    let mut side_builder = VoxelMeshBuilder::new();
    let mut bottom_builder = VoxelMeshBuilder::new();

    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let block = chunk.get_block(x, y, z);
                if block != BlockId::Grass { continue; }

                let render_pos = Vec3::new(x as f32, y as f32, z as f32);

                // 检查每个面是否应该渲染（相邻方块为空或透明）
                let faces_to_render = [
                    (CubeFace::Top, (0i32, 1i32, 0i32)),
                    (CubeFace::Bottom, (0i32, -1i32, 0i32)),
                    (CubeFace::North, (0i32, 0i32, -1i32)),
                    (CubeFace::South, (0i32, 0i32, 1i32)),
                    (CubeFace::East, (1i32, 0i32, 0i32)),
                    (CubeFace::West, (-1i32, 0i32, 0i32)),
                ];

                for (face, (ox, oy, oz)) in faces_to_render {
                    let adjacent_x = x as i32 + ox;
                    let adjacent_y = y as i32 + oy;
                    let adjacent_z = z as i32 + oz;
                    
                    let should_render = if adjacent_x >= 0 && adjacent_x < CHUNK_SIZE as i32 &&
    adjacent_y >= 0 && adjacent_y < CHUNK_SIZE as i32 &&
    adjacent_z >= 0 && adjacent_z < CHUNK_SIZE as i32 {
    chunk.get_block(adjacent_x as u32, adjacent_y as u32, adjacent_z as u32) == BlockId::Air
} else {
    let neighbor_coord = chunk_position + IVec3::new(ox, oy, oz);
    let local_x = if adjacent_x < 0 { adjacent_x + CHUNK_SIZE as i32 } else if adjacent_x >= CHUNK_SIZE as i32 { adjacent_x - CHUNK_SIZE as i32 } else { adjacent_x };
    let local_y = if adjacent_y < 0 { adjacent_y + CHUNK_SIZE as i32 } else if adjacent_y >= CHUNK_SIZE as i32 { adjacent_y - CHUNK_SIZE as i32 } else { adjacent_y };
    let local_z = if adjacent_z < 0 { adjacent_z + CHUNK_SIZE as i32 } else if adjacent_z >= CHUNK_SIZE as i32 { adjacent_z - CHUNK_SIZE as i32 } else { adjacent_z };
    if let Some(neighbor_chunk) = get_neighbor(neighbor_coord) {
        neighbor_chunk.get_block(local_x as u32, local_y as u32, local_z as u32) == BlockId::Air
    } else {
        true
    }
};

                    if should_render {
                        match face {
                            CubeFace::Top => {
                                top_builder.add_cube_face(render_pos, face, 0, true, false); // 翻转UV
                            },
                            CubeFace::Bottom => {
                                bottom_builder.add_cube_face(render_pos, face, 0, false, false);
                            },
                            CubeFace::North | CubeFace::South | CubeFace::East | CubeFace::West => {
                                side_builder.add_cube_face(render_pos, face, 0, false, true); // 垂直翻转UV
                            },
                        }
                    }
                }
            }
        }
    }

    let top_mesh = if !top_builder.positions.is_empty() {
        Some(top_builder.build())
    } else {
        None
    };

    let side_mesh = if !side_builder.positions.is_empty() {
        Some(side_builder.build())
    } else {
        None
    };

    let bottom_mesh = if !bottom_builder.positions.is_empty() {
        Some(bottom_builder.build())
    } else {
        None
    };

    (top_mesh, side_mesh, bottom_mesh)
}

fn get_visible_faces(chunk: &Chunk, x: u32, y: u32, z: u32, chunk_coord: IVec3, get_neighbor: &impl Fn(IVec3) -> Option<Chunk>) -> Vec<CubeFace> {
    let mut faces = Vec::new();
    
    // 检查每个相邻方块 - 只有当相邻位置是空气时才渲染对应面
    let north_visible = if z == 0 {
    if let Some(north_chunk) = get_neighbor(chunk_coord + IVec3::NEG_Z) {
        north_chunk.get_block(x, y, 31) == BlockId::Air
    } else { true }
} else { chunk.get_block(x, y, z - 1) == BlockId::Air };
if north_visible { faces.push(CubeFace::North); }
    let south_visible = if z == CHUNK_SIZE - 1 {
    if let Some(south_chunk) = get_neighbor(chunk_coord + IVec3::Z) {
        south_chunk.get_block(x, y, 0) == BlockId::Air
    } else { true }
} else { chunk.get_block(x, y, z + 1) == BlockId::Air };
if south_visible { faces.push(CubeFace::South); }
    let west_visible = if x == 0 {
    if let Some(west_chunk) = get_neighbor(chunk_coord + IVec3::NEG_X) {
        west_chunk.get_block(31, y, z) == BlockId::Air
    } else { true }
} else { chunk.get_block(x - 1, y, z) == BlockId::Air };
if west_visible { faces.push(CubeFace::West); }
    let east_visible = if x == CHUNK_SIZE - 1 {
    if let Some(east_chunk) = get_neighbor(chunk_coord + IVec3::X) {
        east_chunk.get_block(0, y, z) == BlockId::Air
    } else { true }
} else { chunk.get_block(x + 1, y, z) == BlockId::Air };
if east_visible { faces.push(CubeFace::East); }
    let top_visible = if y == CHUNK_SIZE - 1 {
    if let Some(top_chunk) = get_neighbor(chunk_coord + IVec3::Y) {
        top_chunk.get_block(x, 0, z) == BlockId::Air
    } else { true }
} else { chunk.get_block(x, y + 1, z) == BlockId::Air };
if top_visible { faces.push(CubeFace::Top); }
    let bottom_visible = if y == 0 {
    if let Some(bottom_chunk) = get_neighbor(chunk_coord + IVec3::NEG_Y) {
        bottom_chunk.get_block(x, 31, z) == BlockId::Air
    } else { true }
} else { chunk.get_block(x, y - 1, z) == BlockId::Air };
if bottom_visible { faces.push(CubeFace::Bottom); }
    
    faces
}

fn get_texture_index_for_block(block: BlockId) -> usize {
    match block {
        BlockId::Air => 0,
        BlockId::Stone => 0,
        BlockId::Dirt => 1,
        BlockId::Grass => 2,
        BlockId::Bedrock => 3,
    }
}