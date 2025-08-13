use bevy::prelude::*;
use std::collections::HashMap;
use crate::world::chunk::BlockId;

#[derive(Resource)]
pub struct BlockTextures {
    pub textures: HashMap<BlockId, Handle<Image>>,
    pub materials: HashMap<BlockId, Handle<StandardMaterial>>,
    // 新增：草方块的多面纹理
    pub grass_textures: GrassTextures,
    pub grass_materials: GrassMaterials,
}

// 草方块纹理集合
#[derive(Default)]
pub struct GrassTextures {
    pub top: Option<Handle<Image>>,
    pub side: Option<Handle<Image>>,
    pub bottom: Option<Handle<Image>>,
}

// 草方块材质集合  
#[derive(Default)]
pub struct GrassMaterials {
    pub top: Option<Handle<StandardMaterial>>,
    pub side: Option<Handle<StandardMaterial>>,
    pub bottom: Option<Handle<StandardMaterial>>,
}

#[derive(Resource)]
pub struct TextureLoadingState {
    pub loaded: bool,
}

pub fn load_block_textures(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Loading block textures...");
    
    let mut block_textures = HashMap::new();
    let mut block_materials = HashMap::new();
    
    // 为每种方块类型加载对应纹理
    
    // 石头
    let stone_texture = asset_server.load("textures/block/stone.png");
    let stone_material = materials.add(StandardMaterial {
        base_color_texture: Some(stone_texture.clone()),
        unlit: false,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });
    block_textures.insert(BlockId::Stone, stone_texture);
    block_materials.insert(BlockId::Stone, stone_material);
    
    // 泥土
    let dirt_texture = asset_server.load("textures/block/dirt.png");
    let dirt_material = materials.add(StandardMaterial {
        base_color_texture: Some(dirt_texture.clone()),
        unlit: false,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });
    block_textures.insert(BlockId::Dirt, dirt_texture.clone());
    block_materials.insert(BlockId::Dirt, dirt_material);

    // 基岩
    let bedrock_texture = asset_server.load("textures/block/bedrock.png");
    let bedrock_material = materials.add(StandardMaterial {
        base_color_texture: Some(bedrock_texture.clone()),
        unlit: false,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });
    block_textures.insert(BlockId::Bedrock, bedrock_texture);
    block_materials.insert(BlockId::Bedrock, bedrock_material);

    // 草方块 - 加载多个纹理
    let grass_top_texture = asset_server.load("textures/block/grass_block_top.png");
    let grass_side_texture = asset_server.load("textures/block/grass_block_side.png");
    
    let grass_top_material = materials.add(StandardMaterial {
        base_color_texture: Some(grass_top_texture.clone()),
        unlit: false,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });
    
    let grass_side_material = materials.add(StandardMaterial {
        base_color_texture: Some(grass_side_texture.clone()),
        unlit: false,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });
    
    let grass_bottom_material = materials.add(StandardMaterial {
        base_color_texture: Some(dirt_texture.clone()), // 底部使用泥土纹理
        unlit: false,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });

    // 为了兼容性，草方块的默认纹理和材质设为顶部
    block_textures.insert(BlockId::Grass, grass_top_texture.clone());
    block_materials.insert(BlockId::Grass, grass_top_material.clone());

    // 创建草方块纹理集合
    let grass_textures = GrassTextures {
        top: Some(grass_top_texture),
        side: Some(grass_side_texture),
        bottom: Some(dirt_texture),
    };
    
    let grass_materials = GrassMaterials {
        top: Some(grass_top_material),
        side: Some(grass_side_material),
        bottom: Some(grass_bottom_material),
    };

    // 注意：全局纹理采样（像素化效果）在 main.rs 中通过 ImagePlugin::default_nearest() 进行配置

    commands.insert_resource(BlockTextures {
        textures: block_textures,
        materials: block_materials,
        grass_textures,
        grass_materials,
    });

    commands.insert_resource(TextureLoadingState {
        loaded: true,
    });

    info!("Block textures loaded!");
}