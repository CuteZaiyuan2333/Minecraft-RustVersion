use bevy::prelude::*;

pub mod voxel_mesh;
pub mod texture_loader;

use texture_loader::*;
use voxel_mesh::*;
use crate::world::storage::ChunkStorage;
use crate::game_state::GameState;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), (setup_lighting, load_block_textures))
            .add_systems(Update, update_chunk_meshes.run_if(in_state(GameState::InGame)));
    }
}

fn setup_lighting(mut commands: Commands) {
    // 添加环境光
    commands.insert_resource(AmbientLight {
        color: Color::rgb(0.4, 0.4, 0.45),
        brightness: 0.3,
    });
    
    // 添加方向光（太阳光）
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::rgb(1.0, 0.95, 0.8),
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
        ..default()
    });
}

fn update_chunk_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut chunk_queries: ParamSet<(
        Query<(Entity, &mut crate::world::chunk::Chunk, Option<&ChunkMesh>)>,
        Query<&crate::world::chunk::Chunk>,
    )>,
    block_textures: Option<Res<BlockTextures>>,
    chunk_storage: Res<ChunkStorage>,
) {
    if block_textures.is_none() {
        return; // 纹理还没加载完成
    }
    
    let block_textures = block_textures.unwrap();
    
    // 收集需要更新的chunk信息和数据
    let mut chunks_to_update = Vec::new();
    
    for (entity, chunk, chunk_mesh) in chunk_queries.p0().iter() {
        let needs_update = chunk.dirty || chunk_mesh.is_none();
        if needs_update {
            chunks_to_update.push((entity, chunk.coord, chunk_mesh.is_some(), chunk.clone()));
        }
    }
    
    // 处理需要更新的chunks
    for (entity, coord, has_mesh, chunk_data) in chunks_to_update {
        // 如果已有网格，先清除所有子实体（旧的网格）
        if has_mesh {
            commands.entity(entity).despawn_descendants();
        }
        
        // 计算chunk在世界中的位置
        let chunk_world_pos = Vec3::new(
            (coord.x * 32) as f32,
            (coord.y * 32) as f32,
            (coord.z * 32) as f32,
        );
        
        // 预先收集所有可能需要的邻居chunk数据
        let mut neighbor_chunks = std::collections::HashMap::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let neighbor_coord = coord + IVec3::new(dx, dy, dz);
                    if let Some(neighbor_entity) = chunk_storage.get(&neighbor_coord) {
                        if let Ok(neighbor_chunk) = chunk_queries.p1().get(neighbor_entity) {
                            neighbor_chunks.insert(neighbor_coord, neighbor_chunk.clone());
                        }
                    }
                }
            }
        }
        
        let get_neighbor = |neighbor_coord: IVec3| {
            neighbor_chunks.get(&neighbor_coord).cloned()
        };

        // 为每种方块类型生成单独的网格
         build_and_spawn_chunk_meshes(
             &mut commands,
             entity,
             &chunk_data,
             &mut meshes,
             &block_textures,
             chunk_world_pos,
             get_neighbor,
         );
    }
    
    // 清除所有dirty标志
    for (_, mut chunk, _) in chunk_queries.p0().iter_mut() {
        if chunk.dirty {
            chunk.dirty = false;
        }
    }
}



fn build_and_spawn_chunk_meshes(
    commands: &mut Commands,
    chunk_entity: Entity,
    chunk: &crate::world::chunk::Chunk,
    meshes: &mut ResMut<Assets<Mesh>>,
    block_textures: &BlockTextures,
    _chunk_world_pos: Vec3,
    get_neighbor: impl Fn(IVec3) -> Option<crate::world::chunk::Chunk>,
) {
    use crate::world::chunk::BlockId;
    
    // 首先处理石头、泥土、基岩 - 使用原来的网格构建方式
    let regular_block_types = [BlockId::Stone, BlockId::Dirt, BlockId::Bedrock];
    
    for block_type in regular_block_types {
        let mesh = build_chunk_mesh_for_block_type(chunk, block_type, &get_neighbor);
        
        // 只有当该网格有实际内容时才创建entity
        if mesh.count_vertices() > 0 {
            let mesh_handle = meshes.add(mesh);
            
            // 使用对应方块类型的材质
            if let Some(material_handle) = block_textures.materials.get(&block_type) {
                // 创建子entity来渲染区块网格
                let mesh_entity = commands.spawn((
                    PbrBundle {
                        mesh: mesh_handle,
                        material: material_handle.clone(),
                        transform: Transform::IDENTITY,
                        ..default()
                    },
                )).id();
                
                // 将网格entity设为chunk entity的子entity
                commands.entity(chunk_entity).add_child(mesh_entity);
            }
        }
    }
    
    // 特别处理草方块 - 使用多纹理构建
    let (top_mesh, side_mesh, bottom_mesh) = build_chunk_mesh_for_grass_block(chunk, chunk.coord, block_textures, &get_neighbor);
    
    // 生成草方块的顶面
    if let Some(mesh) = top_mesh {
        let mesh_handle = meshes.add(mesh);
        if let Some(material) = &block_textures.grass_materials.top {
            let mesh_entity = commands.spawn(PbrBundle {
                mesh: mesh_handle,
                material: material.clone(),
                transform: Transform::IDENTITY,
                ..default()
            }).id();
            commands.entity(chunk_entity).add_child(mesh_entity);
        }
    }
    
    // 生成草方块的侧面
    if let Some(mesh) = side_mesh {
        let mesh_handle = meshes.add(mesh);
        if let Some(material) = &block_textures.grass_materials.side {
            let mesh_entity = commands.spawn(PbrBundle {
                mesh: mesh_handle,
                material: material.clone(),
                transform: Transform::IDENTITY,
                ..default()
            }).id();
            commands.entity(chunk_entity).add_child(mesh_entity);
        }
    }
    
    // 生成草方块的底面
    if let Some(mesh) = bottom_mesh {
        let mesh_handle = meshes.add(mesh);
        if let Some(material) = &block_textures.grass_materials.bottom {
            let mesh_entity = commands.spawn(PbrBundle {
                mesh: mesh_handle,
                material: material.clone(),
                transform: Transform::IDENTITY,
                ..default()
            }).id();
            commands.entity(chunk_entity).add_child(mesh_entity);
        }
    }
    
    // 更新chunk mesh状态
    commands.entity(chunk_entity).insert(ChunkMesh {
        coord: chunk.coord,
    });
}