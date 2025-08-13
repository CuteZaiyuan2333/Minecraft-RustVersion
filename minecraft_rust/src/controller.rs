use bevy::prelude::*;
use bevy::input::mouse::{MouseMotion, MouseButton};
use bevy::input::Input;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use crate::world::chunk::{Chunk, BlockId};
use crate::world::storage::ChunkStorage;
use crate::inventory::{PlayerInventory, ItemType, ItemStack};
use crate::game_state::GameState;

#[derive(Debug, Clone, Copy)]
struct AABB {
    min: Vec3,
    max: Vec3,
}

impl AABB {
    fn intersects(&self, other: &AABB) -> bool {
        self.min.x < other.max.x && self.max.x > other.min.x &&
        self.min.y < other.max.y && self.max.y > other.min.y &&
        self.min.z < other.max.z && self.max.z > other.min.z
    }
}

fn get_penetration(player_aabb: &AABB, block_aabb: &AABB) -> Vec3 {
    let overlap_x = (player_aabb.max.x - block_aabb.min.x).min(block_aabb.max.x - player_aabb.min.x);
    let overlap_y = (player_aabb.max.y - block_aabb.min.y).min(block_aabb.max.y - player_aabb.min.y);
    let overlap_z = (player_aabb.max.z - block_aabb.min.z).min(block_aabb.max.z - player_aabb.min.z);
    
    // 找到最小的重叠轴
    if overlap_x < overlap_y && overlap_x < overlap_z {
        // X轴重叠最小
        if player_aabb.min.x < block_aabb.min.x {
            Vec3::new(-overlap_x, 0.0, 0.0)
        } else {
            Vec3::new(overlap_x, 0.0, 0.0)
        }
    } else if overlap_y < overlap_z {
        // Y轴重叠最小
        if player_aabb.min.y < block_aabb.min.y {
            Vec3::new(0.0, -overlap_y, 0.0)
        } else {
            Vec3::new(0.0, overlap_y, 0.0)
        }
    } else {
        // Z轴重叠最小
        if player_aabb.min.z < block_aabb.min.z {
            Vec3::new(0.0, 0.0, -overlap_z)
        } else {
            Vec3::new(0.0, 0.0, overlap_z)
        }
    }
}

fn is_on_ground(position: Vec3, player_height: f32, chunk_storage: &ChunkStorage, chunks: &Query<&Chunk>) -> bool {
    // 增加检测范围到0.2米，提供更好的容错性
    let feet_pos = position - Vec3::new(0.0, 0.2, 0.0);
    let player_size = Vec3::new(0.6, player_height, 0.6);
    
    let player_aabb = AABB { 
        min: feet_pos - Vec3::new(player_size.x / 2.0, 0.0, player_size.z / 2.0), 
        max: feet_pos + Vec3::new(player_size.x / 2.0, 0.2, player_size.z / 2.0) 
    };
    
    // 只检查附近的区块
    let nearby_chunks = get_nearby_chunks(position, chunk_storage, chunks);
    for chunk in nearby_chunks {
        let solids = chunk.get_solid_blocks();
        for &solid in solids {
            let block_world_pos = Vec3::new(
                (chunk.coord.x * 32) as f32 + solid.x as f32,
                (chunk.coord.y * 32) as f32 + solid.y as f32, 
                (chunk.coord.z * 32) as f32 + solid.z as f32,
            );
            let block_aabb = AABB { min: block_world_pos, max: block_world_pos + Vec3::ONE };

            if player_aabb.intersects(&block_aabb) {
                return true;
            }
        }
    }
    false
}

// 优化函数：只检查玩家附近的区块
fn get_nearby_chunks<'a>(position: Vec3, chunk_storage: &ChunkStorage, chunks: &'a Query<&Chunk>) -> Vec<&'a Chunk> {
    let mut nearby_chunks = Vec::new();
    let player_chunk = IVec3::new(
        (position.x / 32.0).floor() as i32,
        (position.y / 32.0).floor() as i32,
        (position.z / 32.0).floor() as i32,
    );
    
    // 只检查玩家周围3x3x3的区块
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                let chunk_coord = player_chunk + IVec3::new(dx, dy, dz);
                if let Some(chunk_entity) = chunk_storage.get(&chunk_coord) {
                    if let Ok(chunk) = chunks.get(chunk_entity) {
                        nearby_chunks.push(chunk);
                    }
                }
            }
        }
    }
    nearby_chunks
}

// 新增函数：检查玩家是否接近地面（用于跳跃检测）
fn is_near_ground(position: Vec3, player_height: f32, chunk_storage: &ChunkStorage, chunks: &Query<&Chunk>) -> bool {
    // 检测脚下0.1米范围内是否有地面，用于跳跃
    let player_size = Vec3::new(0.6, player_height, 0.6);
    
    // 创建一个从玩家脚部向下延伸0.1米的检测区域
    let feet_y = position.y - player_height / 2.0;
    let check_aabb = AABB { 
        min: Vec3::new(
            position.x - player_size.x / 2.0, 
            feet_y - 0.1,  // 向下检测0.1米
            position.z - player_size.z / 2.0
        ), 
        max: Vec3::new(
            position.x + player_size.x / 2.0, 
            feet_y,  // 到脚部位置
            position.z + player_size.z / 2.0
        ) 
    };
    
    // 只检查附近的区块
    let nearby_chunks = get_nearby_chunks(position, chunk_storage, chunks);
    for chunk in nearby_chunks {
        let solids = chunk.get_solid_blocks();
        for &solid in solids {
            let block_world_pos = Vec3::new(
                (chunk.coord.x * 32) as f32 + solid.x as f32,
                (chunk.coord.y * 32) as f32 + solid.y as f32, 
                (chunk.coord.z * 32) as f32 + solid.z as f32,
            );
            let block_aabb = AABB { min: block_world_pos, max: block_world_pos + Vec3::ONE };

            if check_aabb.intersects(&block_aabb) {
                return true;
            }
        }
    }
    false
}

fn world_pos_to_chunk_coord(world_pos: IVec3) -> IVec3 {
    IVec3::new(
        world_pos.x.div_euclid(32),
        world_pos.y.div_euclid(32),
        world_pos.z.div_euclid(32),
    )
}

fn world_pos_to_local_pos(world_pos: IVec3, chunk_coord: IVec3) -> IVec3 {
    world_pos - chunk_coord * 32
}

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            handle_mouse_look,
            handle_movement,
            handle_cursor_grab,
            handle_block_interaction,
        ).run_if(in_state(GameState::InGame)));
    }
}

#[derive(Component)]
pub struct FirstPersonController {
    pub speed: f32,
    pub sensitivity: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub mode: ControlMode,
    pub velocity: Vec3,
    pub last_space_time: f64,
    pub is_sneaking: bool,
    // 新增参数改善手感
    pub acceleration: f32,        // 地面加速度
    pub air_acceleration: f32,    // 空中加速度
    pub friction: f32,            // 地面摩擦力
    pub air_friction: f32,        // 空中阻力
    pub max_speed: f32,           // 最大移动速度
    pub sprint_multiplier: f32,   // 冲刺速度倍数
    pub is_sprinting: bool,       // 是否在冲刺
}

#[derive(PartialEq)]
pub enum ControlMode {
    Flying,
    Walking,
}

impl Default for FirstPersonController {
    fn default() -> Self {
        Self {
            speed: 5.0,
            sensitivity: 0.002,
            yaw: 0.0,
            pitch: 0.0,
            mode: ControlMode::Walking,
            velocity: Vec3::ZERO,
            last_space_time: 0.0,
            is_sneaking: false,
            // 新增参数的默认值
            acceleration: 20.0,        // 地面加速度
            air_acceleration: 8.0,     // 空中加速度（较低）
            friction: 12.0,            // 地面摩擦力
            air_friction: 0.5,         // 空中阻力（很小）
            max_speed: 8.0,            // 最大移动速度
            sprint_multiplier: 1.6,    // 冲刺速度倍数
            is_sprinting: false,       // 默认不冲刺
        }
    }
}

fn handle_block_interaction(
    mouse_buttons: Res<Input<MouseButton>>,
    mut controller_query: Query<(&FirstPersonController, &Transform, &Children, &mut PlayerInventory)>,
    camera_query: Query<&Transform, (With<Camera3d>, Without<FirstPersonController>)>,
    mut chunk_query: Query<&mut Chunk>,
    chunk_storage: Res<ChunkStorage>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
) {
    let window = primary_window.single();
    if window.cursor.grab_mode != CursorGrabMode::Locked {
        return;
    }

    let left_clicked = mouse_buttons.just_pressed(MouseButton::Left);
    let right_clicked = mouse_buttons.just_pressed(MouseButton::Right);
    
    if !left_clicked && !right_clicked {
        return;
    }

    if let Ok((_, player_transform, children, mut inventory)) = controller_query.get_single_mut() {
        // 找到摄像机并获取其全局变换
        let mut camera_global_transform = None;
        for &child in children.iter() {
            if let Ok(camera_transform) = camera_query.get(child) {
                // 计算摄像机的全局变换（玩家变换 + 摄像机本地变换）
                let global_camera_transform = player_transform.mul_transform(*camera_transform);
                camera_global_transform = Some(global_camera_transform);
                break;
            }
        }

        if let Some(camera_transform) = camera_global_transform {
            let ray_origin = camera_transform.translation;
            let ray_direction = camera_transform.forward();
            
            println!("射线起点: {:?}, 方向: {:?}", ray_origin, ray_direction);
            
            // 增加交互距离到8.0，让玩家可以"手再长一点"
            if let Some((hit_block_pos, face_normal)) = raycast_for_blocks(
                ray_origin, 
                ray_direction, 
                8.0,  // 从5.0增加到8.0
                &chunk_query,
                &chunk_storage
            ) {
                if left_clicked {
                    // 破坏方块
                    destroy_block(hit_block_pos, &mut chunk_query, &chunk_storage);
                } else if right_clicked {
                    // 放置方块 - 使用物品栏中选中的物品
                    let selected_item = inventory.get_selected_item();
                    if let ItemType::Block(block_id) = selected_item.item_type {
                        if selected_item.count > 0 {
                            let place_pos = hit_block_pos + face_normal;
                            
                            // 检查是否与玩家重叠（考虑玩家高度1.8米）
                            let player_block_pos = IVec3::new(
                                player_transform.translation.x.floor() as i32,
                                player_transform.translation.y.floor() as i32,
                                player_transform.translation.z.floor() as i32,
                            );
                            let player_head_pos = player_block_pos + IVec3::Y;
                            
                            if place_pos != player_block_pos && place_pos != player_head_pos {
                                place_block(place_pos, block_id, &mut chunk_query, &chunk_storage);
                                
                                // 消耗物品栏中的物品
                                let selected_item_mut = inventory.get_selected_item_mut();
                                selected_item_mut.count -= 1;
                                if selected_item_mut.count == 0 {
                                    *selected_item_mut = ItemStack::empty();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn raycast_for_blocks(
    ray_origin: Vec3,
    ray_direction: Vec3,
    max_distance: f32,
    chunk_query: &Query<&mut Chunk>,
    chunk_storage: &ChunkStorage,
) -> Option<(IVec3, IVec3)> {
    // 使用改进的DDA算法进行精确的体素遍历
    let current_pos = ray_origin;
    let mut current_block = IVec3::new(
        current_pos.x.floor() as i32,
        current_pos.y.floor() as i32,
        current_pos.z.floor() as i32,
    );
    
    // 计算射线方向的符号和步长
    let step_x = if ray_direction.x > 0.0 { 1 } else { -1 };
    let step_y = if ray_direction.y > 0.0 { 1 } else { -1 };
    let step_z = if ray_direction.z > 0.0 { 1 } else { -1 };
    
    // 避免除零错误
    let delta_x = if ray_direction.x.abs() < 1e-6 { f32::INFINITY } else { (1.0 / ray_direction.x).abs() };
    let delta_y = if ray_direction.y.abs() < 1e-6 { f32::INFINITY } else { (1.0 / ray_direction.y).abs() };
    let delta_z = if ray_direction.z.abs() < 1e-6 { f32::INFINITY } else { (1.0 / ray_direction.z).abs() };
    
    // 计算到下一个网格线的距离
    let mut max_x = if ray_direction.x > 0.0 {
        delta_x * (current_block.x as f32 + 1.0 - current_pos.x)
    } else {
        delta_x * (current_pos.x - current_block.x as f32)
    };
    
    let mut max_y = if ray_direction.y > 0.0 {
        delta_y * (current_block.y as f32 + 1.0 - current_pos.y)
    } else {
        delta_y * (current_pos.y - current_block.y as f32)
    };
    
    let mut max_z = if ray_direction.z > 0.0 {
        delta_z * (current_block.z as f32 + 1.0 - current_pos.z)
    } else {
        delta_z * (current_pos.z - current_block.z as f32)
    };
    
    let mut distance_traveled = 0.0;
    let mut last_face_normal = IVec3::ZERO;
    
    // DDA主循环
    while distance_traveled < max_distance {
        // 检查当前方块是否为实心
        if is_solid_block(current_block, chunk_query, chunk_storage) {
            println!("射线击中方块: 世界坐标 {:?}, 面法线 {:?}", current_block, last_face_normal);
            return Some((current_block, last_face_normal));
        }
        
        // 移动到下一个方块
        if max_x < max_y && max_x < max_z {
            // X方向最近
            distance_traveled = max_x;
            max_x += delta_x;
            current_block.x += step_x;
            last_face_normal = IVec3::new(-step_x, 0, 0);
        } else if max_y < max_z {
            // Y方向最近
            distance_traveled = max_y;
            max_y += delta_y;
            current_block.y += step_y;
            last_face_normal = IVec3::new(0, -step_y, 0);
        } else {
            // Z方向最近
            distance_traveled = max_z;
            max_z += delta_z;
            current_block.z += step_z;
            last_face_normal = IVec3::new(0, 0, -step_z);
        }
    }
    
    None
}



fn is_solid_block(
    world_pos: IVec3,
    chunk_query: &Query<&mut Chunk>,
    chunk_storage: &ChunkStorage,
) -> bool {
    let chunk_coord = world_pos_to_chunk_coord(world_pos);
    
    if let Some(chunk_entity) = chunk_storage.get(&chunk_coord) {
        if let Ok(chunk) = chunk_query.get(chunk_entity) {
            let local_pos = world_pos_to_local_pos(world_pos, chunk_coord);
            
            // 确保坐标在有效范围内
            if local_pos.x >= 0 && local_pos.x < 32 &&
               local_pos.y >= 0 && local_pos.y < 32 &&
               local_pos.z >= 0 && local_pos.z < 32 {
                let block = chunk.get_block(local_pos.x as u32, local_pos.y as u32, local_pos.z as u32);
                return block != BlockId::Air;
            }
        }
    }
    
    false
}

fn destroy_block(
    world_pos: IVec3,
    chunk_query: &mut Query<&mut Chunk>,
    chunk_storage: &ChunkStorage,
) {
    let chunk_coord = world_pos_to_chunk_coord(world_pos);
    
    if let Some(chunk_entity) = chunk_storage.get(&chunk_coord) {
        if let Ok(mut chunk) = chunk_query.get_mut(chunk_entity) {
            let local_pos = world_pos_to_local_pos(world_pos, chunk_coord);
            
            if local_pos.x >= 0 && local_pos.x < 32 &&
               local_pos.y >= 0 && local_pos.y < 32 &&
               local_pos.z >= 0 && local_pos.z < 32 {
                
                println!("破坏方块: 世界坐标 {:?}, chunk {:?}, 本地坐标 {:?}", 
                        world_pos, chunk_coord, local_pos);
                
                chunk.set_block(local_pos.x as u32, local_pos.y as u32, local_pos.z as u32, BlockId::Air);
                chunk.compute_solid_blocks();
                chunk.dirty = true;
                
                // 标记相邻区块为脏，如果方块在区块边界
                mark_neighbor_chunks_dirty(world_pos, local_pos, chunk_query, chunk_storage);
            }
        }
    }
}

fn place_block(
    world_pos: IVec3,
    block_id: BlockId,
    chunk_query: &mut Query<&mut Chunk>,
    chunk_storage: &ChunkStorage,
) {
    let chunk_coord = world_pos_to_chunk_coord(world_pos);
    
    if let Some(chunk_entity) = chunk_storage.get(&chunk_coord) {
        if let Ok(mut chunk) = chunk_query.get_mut(chunk_entity) {
            let local_pos = world_pos_to_local_pos(world_pos, chunk_coord);
            
            if local_pos.x >= 0 && local_pos.x < 32 &&
               local_pos.y >= 0 && local_pos.y < 32 &&
               local_pos.z >= 0 && local_pos.z < 32 {
                
                println!("放置方块: 世界坐标 {:?}, chunk {:?}, 本地坐标 {:?}, 类型 {:?}", 
                        world_pos, chunk_coord, local_pos, block_id);
                
                chunk.set_block(local_pos.x as u32, local_pos.y as u32, local_pos.z as u32, block_id);
                chunk.compute_solid_blocks();
                chunk.dirty = true;
                
                // 标记相邻区块为脏，如果方块在区块边界
                mark_neighbor_chunks_dirty(world_pos, local_pos, chunk_query, chunk_storage);
            }
        }
    }
}

// 新增函数：标记相邻区块为脏
fn mark_neighbor_chunks_dirty(
    world_pos: IVec3,
    local_pos: IVec3,
    chunk_query: &mut Query<&mut Chunk>,
    chunk_storage: &ChunkStorage,
) {
    // 检查是否在区块边界
    let neighbors = [
        (local_pos.x == 0, IVec3::new(-1, 0, 0)),   // 左边界
        (local_pos.x == 31, IVec3::new(1, 0, 0)),   // 右边界
        (local_pos.y == 0, IVec3::new(0, -1, 0)),   // 下边界
        (local_pos.y == 31, IVec3::new(0, 1, 0)),   // 上边界
        (local_pos.z == 0, IVec3::new(0, 0, -1)),   // 前边界
        (local_pos.z == 31, IVec3::new(0, 0, 1)),   // 后边界
    ];
    
    let current_chunk_coord = world_pos_to_chunk_coord(world_pos);
    
    for (is_boundary, offset) in neighbors {
        if is_boundary {
            let neighbor_chunk_coord = current_chunk_coord + offset;
            if let Some(neighbor_entity) = chunk_storage.get(&neighbor_chunk_coord) {
                if let Ok(mut neighbor_chunk) = chunk_query.get_mut(neighbor_entity) {
                    neighbor_chunk.dirty = true;
                    println!("标记相邻区块为脏: {:?}", neighbor_chunk_coord);
                }
            }
        }
    }
}



fn handle_mouse_look(
    mut mouse_motion: EventReader<MouseMotion>,
    mut controller_query: Query<(&mut FirstPersonController, &mut Transform, &Children)>,
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<FirstPersonController>)>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
    keyboard: Res<Input<KeyCode>>,
    game_settings: Res<crate::ui::GameSettings>,
) {
    let mut window = primary_window.single_mut();
    if window.cursor.grab_mode != CursorGrabMode::Locked {
        return;
    }

    // 检查是否按住ALT键，如果是则不处理鼠标视角
    if keyboard.pressed(KeyCode::AltLeft) || keyboard.pressed(KeyCode::AltRight) {
        return;
    }

    let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);

    for (mut controller, mut player_transform, children) in controller_query.iter_mut() {
        for motion in mouse_motion.read() {
            // 使用游戏设置中的鼠标灵敏度
            let effective_sensitivity = controller.sensitivity * game_settings.mouse_sensitivity;
            
            // 更新yaw和pitch
            controller.yaw -= motion.delta.x * effective_sensitivity;
            controller.pitch -= motion.delta.y * effective_sensitivity;
            
            // 限制pitch范围
            controller.pitch = controller.pitch.clamp(-1.54, 1.54); // ~88度
            
            // 只有yaw影响玩家身体旋转（左右转身）
            player_transform.rotation = Quat::from_axis_angle(Vec3::Y, controller.yaw);
            
            // pitch只影响摄像机（上下看）
            for &child in children.iter() {
                if let Ok(mut camera_transform) = camera_query.get_mut(child) {
                    camera_transform.rotation = Quat::from_axis_angle(Vec3::X, controller.pitch);
                }
            }
        }
    }

    // 每帧（有鼠标事件时）将系统光标重置到窗口中心，实现"锁定在中心"的效果
    window.set_cursor_position(Some(center));
}

fn handle_movement(
    mut query: Query<(&mut Transform, &mut FirstPersonController)>,
    keyboard: Res<Input<KeyCode>>,
    time: Res<Time>,
    chunks: Query<&Chunk>,
    chunk_storage: Res<ChunkStorage>,
    game_settings: Res<crate::ui::GameSettings>,
) {
    for (mut transform, mut controller) in query.iter_mut() {
        let mut input_direction = Vec3::ZERO;
        
        // 获取摄像机的前向和右向向量
        let forward = -transform.local_z();
        let right = transform.local_x();
        
        // 处理输入
        if keyboard.pressed(KeyCode::W) { input_direction += forward; }
        if keyboard.pressed(KeyCode::S) { input_direction -= forward; }
        if keyboard.pressed(KeyCode::A) { input_direction -= right; }
        if keyboard.pressed(KeyCode::D) { input_direction += right; }
        
        // 归一化水平移动向量（保持Y为0）
        input_direction.y = 0.0;
        if input_direction.length_squared() > 0.0 {
            input_direction = input_direction.normalize();
        }
        
        // 检查冲刺状态
        controller.is_sprinting = keyboard.pressed(KeyCode::ControlLeft);
        
        // 潜行状态
        controller.is_sneaking = keyboard.pressed(KeyCode::ShiftLeft);
        
        // 根据潜行状态调整摄像机和玩家高度
        let player_height = if controller.is_sneaking { 1.5 } else { 1.8 };
        
        if controller.mode == ControlMode::Flying {
            // 飞行模式处理双击空格切换
            if keyboard.just_pressed(KeyCode::Space) {
                let current_time = time.elapsed_seconds_f64();
                if current_time - controller.last_space_time < 0.3 {
                    controller.mode = ControlMode::Walking;
                    controller.velocity = Vec3::ZERO;
                    controller.last_space_time = current_time;
                    continue;
                }
                controller.last_space_time = current_time;
            }
            
            // 飞行移动（保持原有逻辑）
            if keyboard.pressed(KeyCode::Space) { input_direction.y += 1.0; }
            if keyboard.pressed(KeyCode::ShiftLeft) { input_direction.y -= 1.0; }
            
            if input_direction.length_squared() > 0.0 {
                controller.velocity = input_direction.normalize() * controller.speed;
            } else {
                controller.velocity = Vec3::ZERO;
            }
        } else { // 行走模式 - 新的移动逻辑
            // 重力 - 使用设置中的重力值，乘以2增强下落感
            controller.velocity.y -= game_settings.gravity * 2.0 * time.delta_seconds();

            // 地面检测 - 使用更宽松的检测减少抖动
            let on_ground = is_on_ground(transform.translation, player_height, &chunk_storage, &chunks);
            
            // 如果在地面上且垂直速度向下，将其设为0以减少抖动
            if on_ground && controller.velocity.y < 0.0 {
                controller.velocity.y = 0.0;
            }

            // 计算目标速度
            let mut target_speed = controller.speed;
            if controller.is_sneaking {
                target_speed *= 0.3; // 潜行速度为30%
            } else if controller.is_sprinting {
                target_speed *= controller.sprint_multiplier; // 冲刺速度
            }
            
            // 限制最大速度
            target_speed = target_speed.min(controller.max_speed);

            // 水平移动处理
            let delta_time = time.delta_seconds();
            let current_horizontal_velocity = Vec2::new(controller.velocity.x, controller.velocity.z);
            
            if input_direction.xz().length_squared() > 0.0 {
                // 有输入时
                let target_velocity = input_direction.xz().normalize() * target_speed;
                let acceleration = if on_ground { controller.acceleration } else { controller.air_acceleration };
                
                // 使用加速度平滑过渡到目标速度
                let velocity_diff = target_velocity - current_horizontal_velocity;
                let max_velocity_change = acceleration * delta_time;
                
                let new_horizontal_velocity = if velocity_diff.length() <= max_velocity_change {
                    target_velocity
                } else {
                    current_horizontal_velocity + velocity_diff.normalize() * max_velocity_change
                };
                
                controller.velocity.x = new_horizontal_velocity.x;
                controller.velocity.z = new_horizontal_velocity.y;
            } else {
                // 无输入时应用摩擦力
                let friction = if on_ground { controller.friction } else { controller.air_friction };
                let friction_force = current_horizontal_velocity * friction * delta_time;
                
                if friction_force.length() >= current_horizontal_velocity.length() {
                    // 摩擦力足够大，直接停止
                    controller.velocity.x = 0.0;
                    controller.velocity.z = 0.0;
                } else {
                    // 应用摩擦力
                    let new_horizontal_velocity = current_horizontal_velocity - friction_force;
                    controller.velocity.x = new_horizontal_velocity.x;
                    controller.velocity.z = new_horizontal_velocity.y;
                }
            }
        }

        // 应用速度
        let delta_time = time.delta_seconds();
        let mut proposed_pos = transform.translation + controller.velocity * delta_time;

        // 碰撞检测和处理 - 使用优化的附近区块检测
        let player_size = Vec3::new(0.6, player_height, 0.6);
        
        let player_aabb = AABB { 
            min: proposed_pos - Vec3::new(player_size.x / 2.0, 0.0, player_size.z / 2.0), 
            max: proposed_pos + Vec3::new(player_size.x / 2.0, player_size.y, player_size.z / 2.0) 
        };
        
        // 只检查玩家附近的区块，提高性能
        let nearby_chunks = get_nearby_chunks(proposed_pos, &chunk_storage, &chunks);
        for chunk in nearby_chunks {
            let solids = chunk.get_solid_blocks();
            for &solid in solids {
                let block_world_pos = Vec3::new(
                    (chunk.coord.x * 32) as f32 + solid.x as f32,
                    (chunk.coord.y * 32) as f32 + solid.y as f32, 
                    (chunk.coord.z * 32) as f32 + solid.z as f32,
                );
                let block_aabb = AABB { min: block_world_pos, max: block_world_pos + Vec3::ONE };

                if player_aabb.intersects(&block_aabb) {
                    let penetration = get_penetration(&player_aabb, &block_aabb);
                    proposed_pos += penetration;
                    
                    if penetration.y.abs() > penetration.x.abs() && penetration.y.abs() > penetration.z.abs() {
                        // 垂直碰撞
                        if controller.mode == ControlMode::Walking {
                            // 只有在向下移动时才重置垂直速度（着陆）
                            // 或者在向上移动时撞到天花板
                            if (penetration.y > 0.0 && controller.velocity.y <= 0.0) ||
                               (penetration.y < 0.0 && controller.velocity.y >= 0.0) {
                                controller.velocity.y = 0.0;
                            }
                        } else {
                            controller.velocity.y = 0.0;
                        }
                    } else {
                        // 水平碰撞
                        if penetration.x.abs() > penetration.z.abs() {
                            controller.velocity.x = 0.0;
                        } else {
                            controller.velocity.z = 0.0;
                        }
                    }
                }
            }
        }

        transform.translation = proposed_pos;

        // 跳跃和飞行切换
        if controller.mode == ControlMode::Walking && keyboard.just_pressed(KeyCode::Space) {
            let current_time = time.elapsed_seconds_f64();
            if current_time - controller.last_space_time < 0.3 {
                // 双击空格 - 切换到飞行
                controller.mode = ControlMode::Flying;
                controller.velocity = Vec3::ZERO;
            } else if is_near_ground(transform.translation, player_height, &chunk_storage, &chunks) {
                // 单击空格且接近地面 - 跳跃（允许在距离地面0.1米内跳跃）
                controller.velocity.y = 6.6; // 适应重力*2的跳跃速度，能跳到1.1格高度
            }
            controller.last_space_time = current_time;
        }
    }
}



fn handle_cursor_grab(
    mouse_buttons: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mut window = primary_window.single_mut();

    // 鼠标左键点击窗口后自动锁定并隐藏光标，但按住ALT时不锁定
    if mouse_buttons.just_pressed(MouseButton::Left) && 
       !keyboard.pressed(KeyCode::AltLeft) && 
       !keyboard.pressed(KeyCode::AltRight) {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
        // 居中系统鼠标位置，避免锁定前存在偏移
        let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        window.set_cursor_position(Some(center));
    }

    // 按住 Alt 键时，临时解锁鼠标（释放即可继续锁定）
    if keyboard.pressed(KeyCode::AltLeft) || keyboard.pressed(KeyCode::AltRight) {
        if window.cursor.grab_mode == CursorGrabMode::Locked {
            window.cursor.grab_mode = CursorGrabMode::None;
            window.cursor.visible = true;
        }
    } else {
        // 松开 Alt 后自动回到锁定，同时再次居中
        if window.cursor.visible && window.cursor.grab_mode == CursorGrabMode::None {
            window.cursor.grab_mode = CursorGrabMode::Locked;
            window.cursor.visible = false;
            let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
            window.set_cursor_position(Some(center));
        }
    }
}