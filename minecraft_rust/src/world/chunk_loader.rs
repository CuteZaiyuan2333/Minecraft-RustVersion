use bevy::prelude::*;
use std::collections::{HashSet, VecDeque};
use crate::world::chunk::Chunk;
use crate::world::storage::ChunkStorage;
use crate::world::generator::{WorldGenerator, WorldGeneratorConfig};
use crate::block_registry::BlockRegistry;
use crate::controller::FirstPersonController;
use bevy::tasks::{AsyncComputeTaskPool, Task, TaskPool, TaskPoolBuilder};
use futures_lite::future;
use crate::game_state::GameState;
use crate::ui::GameSettings;
use std::sync::{Arc, Mutex};

/// 区块加载器配置
#[derive(Resource)]
pub struct ChunkLoaderConfig {
    pub max_loaded_chunks: usize,    // 最大同时加载区块数量
    pub surface_priority_quota: usize, // 地表优先区块配额
    pub sphere_loading_radius: f32,   // 球形加载半径
    pub max_chunks_per_frame: usize, // 每帧最多处理的区块数量
}

impl Default for ChunkLoaderConfig {
    fn default() -> Self {
        Self {
            max_loaded_chunks: 1000,     // 默认最大1000个区块
            surface_priority_quota: 600, // 地表优先配额600个
            sphere_loading_radius: 12.0, // 球形加载半径12个区块
            max_chunks_per_frame: 3,     // 每帧最多处理3个区块
        }
    }
}

/// 异步区块生成任务
#[derive(Component)]
pub struct ChunkGenerationTask {
    pub task: Task<Chunk>,
    pub position: IVec3,
}

/// 异步区块卸载任务
#[derive(Component)]
pub struct ChunkUnloadTask {
    pub task: Task<()>,
    pub position: IVec3,
    pub entity: Entity,
}

/// 区块加载队列
#[derive(Resource, Default)]
pub struct ChunkLoadQueue {
    pub pending: VecDeque<IVec3>,  // 待加载的区块位置
    pub generating: HashSet<IVec3>,  // 正在生成的区块位置
}

/// 区块卸载队列
#[derive(Resource, Default)]
pub struct ChunkUnloadQueue {
    pub pending: VecDeque<(Entity, IVec3)>,  // 待卸载的区块
    pub unloading: HashSet<IVec3>,  // 正在卸载的区块位置
}

/// 自定义区块生成线程池
#[derive(Resource)]
pub struct ChunkGenerationThreadPool {
    pub pool: Arc<TaskPool>,
    pub thread_count: u32,
}

impl ChunkGenerationThreadPool {
    pub fn new(thread_count: u32) -> Self {
        let thread_count = thread_count.max(1);
        info!("Creating chunk generation thread pool with {} threads", thread_count);
        
        // 使用 TaskPoolBuilder 来创建指定线程数的线程池
        let pool = TaskPoolBuilder::new()
            .num_threads(thread_count as usize)
            .thread_name("chunk_generation".to_string())
            .build();
        Self {
            pool: Arc::new(pool),
            thread_count,
        }
    }
    
    pub fn update_thread_count(&mut self, new_count: u32) {
        let new_count = new_count.max(1);
        if new_count != self.thread_count {
            info!("Updating chunk generation thread pool from {} to {} threads", self.thread_count, new_count);
            self.thread_count = new_count;
            // 重新创建线程池
            let pool = TaskPoolBuilder::new()
                .num_threads(new_count as usize)
                .thread_name("chunk_generation".to_string())
                .build();
            self.pool = Arc::new(pool);
        }
    }
}

/// 线程池管理系统 - 监控设置变化并更新线程池
pub fn thread_pool_management_system(
    mut thread_pool: ResMut<ChunkGenerationThreadPool>,
    game_settings: Option<Res<GameSettings>>,
) {
    if let Some(settings) = game_settings {
        if settings.is_changed() {
            thread_pool.update_thread_count(settings.chunk_generation_threads);
        }
    }
}

/// 智能区块需求分析系统 - 基于数量限制的智能加载策略
pub fn chunk_demand_system(
    player_query: Query<&Transform, With<FirstPersonController>>,
    mut loader_config: ResMut<ChunkLoaderConfig>,
    game_settings: Option<Res<GameSettings>>,
    mut load_queue: ResMut<ChunkLoadQueue>,
    chunk_query: Query<&Chunk>,
    time: Res<Time>,
) {
    // 从游戏设置更新配置
    if let Some(settings) = game_settings {
        loader_config.max_loaded_chunks = settings.max_loaded_chunks as usize;
        loader_config.surface_priority_quota = settings.surface_priority_quota as usize;
        loader_config.sphere_loading_radius = settings.sphere_loading_radius;
    }
    
    // 添加静态变量来缓存上次检查的时间和位置，以及深度地下检测
    static LAST_CHECK: Mutex<Option<(f32, IVec3, Vec3)>> = Mutex::new(None);
    static DEEP_UNDERGROUND_TIMER: Mutex<Option<f32>> = Mutex::new(None); // 深度地下计时器
    
    // 获取玩家位置
    let player_transform = match player_query.get_single() {
        Ok(transform) => transform,
        Err(_) => return,
    };

    let player_pos = player_transform.translation;
    let player_chunk_pos = IVec3::new(
        (player_pos.x / 32.0).floor() as i32,
        (player_pos.y / 32.0).floor() as i32,
        (player_pos.z / 32.0).floor() as i32,
    );

    // 检查是否需要更新，并检测快速移动
    let current_time = time.elapsed_seconds();
    let mut should_update = false;
    let mut is_fast_moving = false;
    let mut emergency_load = false;
    let mut player_velocity = Vec3::ZERO; // 初始化玩家速度
    
    if let Ok(mut last_check) = LAST_CHECK.lock() {
        if let Some((last_time, last_chunk_pos, last_world_pos)) = *last_check {
            let time_delta = current_time - last_time;
            let chunk_moved = last_chunk_pos != player_chunk_pos;
            
            // 计算移动速度和速度向量
            let distance_moved = player_pos.distance(last_world_pos);
            let speed = if time_delta > 0.0 { distance_moved / time_delta } else { 0.0 };
            
            // 计算速度向量
            if time_delta > 0.0 {
                player_velocity = (player_pos - last_world_pos) / time_delta;
            }
            
            // 检测快速移动（速度超过30单位/秒，或Y轴快速下降超过10单位）
             is_fast_moving = speed > 30.0 || (player_pos.y - last_world_pos.y) < -10.0;
            
            // 紧急加载条件：快速移动且移动到新区块
            emergency_load = is_fast_moving && chunk_moved;
            
            // 更新条件：时间间隔或移动到新区块
            if time_delta > 0.5 || chunk_moved || emergency_load {
                should_update = true;
                *last_check = Some((current_time, player_chunk_pos, player_pos));
            }
        } else {
            should_update = true;
            *last_check = Some((current_time, player_chunk_pos, player_pos));
        }
    }
    
    if !should_update {
        return;
    }

    // 收集当前已加载的区块
    let mut loaded_chunks = HashSet::new();
    for chunk in chunk_query.iter() {
        loaded_chunks.insert(chunk.coord);
    }

    // 检查是否达到最大区块数量限制（快速移动时大幅放宽限制）
    let current_loaded_count = loaded_chunks.len();
    
    // 异步检测算法：简化检测逻辑，减少主线程计算
    let is_near_surface_simple = player_chunk_pos.y >= 0;
    let is_underground_simple = player_chunk_pos.y < 0;
    
    // 深度地下检测：检查玩家周围8个区块是否都不属于地表
    let surrounding_chunks = vec![
        IVec3::new(player_chunk_pos.x + 1, player_chunk_pos.y, player_chunk_pos.z),     // 东
        IVec3::new(player_chunk_pos.x - 1, player_chunk_pos.y, player_chunk_pos.z),     // 西
        IVec3::new(player_chunk_pos.x, player_chunk_pos.y, player_chunk_pos.z + 1),     // 南
        IVec3::new(player_chunk_pos.x, player_chunk_pos.y, player_chunk_pos.z - 1),     // 北
        IVec3::new(player_chunk_pos.x + 1, player_chunk_pos.y, player_chunk_pos.z + 1), // 东南
        IVec3::new(player_chunk_pos.x + 1, player_chunk_pos.y, player_chunk_pos.z - 1), // 东北
        IVec3::new(player_chunk_pos.x - 1, player_chunk_pos.y, player_chunk_pos.z + 1), // 西南
        IVec3::new(player_chunk_pos.x - 1, player_chunk_pos.y, player_chunk_pos.z - 1), // 西北
    ];
    
    let all_chunks_underground = surrounding_chunks.iter().all(|chunk_pos| chunk_pos.y < 0);
    
    // 深度地下计时器管理
    let mut is_deep_underground_long_time = false;
    if let Ok(mut timer) = DEEP_UNDERGROUND_TIMER.lock() {
        if all_chunks_underground {
            // 开始或继续计时
            if timer.is_none() {
                *timer = Some(current_time);
            } else if let Some(start_time) = *timer {
                // 检查是否已经持续30秒
                if current_time - start_time >= 30.0 {
                    is_deep_underground_long_time = true;
                }
            }
        } else {
            // 重置计时器
            *timer = None;
        }
    }
    
    // 保守的500区块限制：如果两个检测都不为真，则限制为500个区块
    let conservative_limit = 500;
    let use_conservative_mode = !is_near_surface_simple && !emergency_load && !is_fast_moving;
    
    let effective_max = if is_deep_underground_long_time {
        // 深度地下激进模式：只保留最少的必要区块
        50 // 激进模式：只保留50个区块
    } else if use_conservative_mode {
        conservative_limit.min(loader_config.max_loaded_chunks) // 保守模式：最多500个区块
    } else if emergency_load {
        loader_config.max_loaded_chunks + 200 // 紧急情况下允许超出200个区块
    } else if is_fast_moving {
        loader_config.max_loaded_chunks + 100 // 快速移动时允许超出100个区块
    } else {
        loader_config.max_loaded_chunks
    };
    
    if current_loaded_count >= effective_max {
        if use_conservative_mode {
            info!("Conservative mode: {} loaded, limited to {}, surface: {}, emergency: {}, fast_moving: {}", 
                   current_loaded_count, effective_max, is_near_surface_simple, emergency_load, is_fast_moving);
        } else {
            debug!("Max loaded chunks reached: {}/{} (emergency: {}, fast_moving: {})", 
                   current_loaded_count, effective_max, emergency_load, is_fast_moving);
        }
        return; // 已达到限制，等待卸载系统释放空间
    }

    // 计算可用的加载配额
    let available_quota = effective_max - current_loaded_count;
    
    // 智能脚下区块保护：永远优先加载玩家脚下的三个区块
     let mut emergency_chunks = Vec::new();
     
     // 第一优先级：永远加载玩家脚下的三个区块（无条件）
     let critical_foot_chunks = vec![
         IVec3::new(player_chunk_pos.x, player_chunk_pos.y - 1, player_chunk_pos.z), // 脚下第一层
         IVec3::new(player_chunk_pos.x, player_chunk_pos.y - 2, player_chunk_pos.z), // 脚下第二层
         IVec3::new(player_chunk_pos.x, player_chunk_pos.y - 3, player_chunk_pos.z), // 脚下第三层
         player_chunk_pos, // 玩家当前区块
     ];
     
     for chunk_pos in critical_foot_chunks {
         if !loaded_chunks.contains(&chunk_pos) && !load_queue.generating.contains(&chunk_pos) {
             emergency_chunks.push((chunk_pos, 0.0)); // 最高优先级
         }
     }
     
     // 检测持续下落：如果玩家Y速度持续向下，立即加载更多脚下区块
     let is_falling_fast = player_velocity.y < -5.0; // 快速下落检测
     if is_falling_fast {
         // 下落时加载更多脚下区块
         for i in 4..=8 {
             let chunk_pos = IVec3::new(player_chunk_pos.x, player_chunk_pos.y - i, player_chunk_pos.z);
             if !loaded_chunks.contains(&chunk_pos) && !load_queue.generating.contains(&chunk_pos) {
                 emergency_chunks.push((chunk_pos, 0.1)); // 下落保护优先级
             }
         }
     }
     
     // 第二优先级：紧急加载时的周围核心区块
     if emergency_load {
         let emergency_radius = if is_falling_fast { 1 } else { 2 }; // 下落时减少水平范围
         for x in (player_chunk_pos.x - emergency_radius)..=(player_chunk_pos.x + emergency_radius) {
             for y in (player_chunk_pos.y - 1)..=(player_chunk_pos.y + 1) {
                 for z in (player_chunk_pos.z - emergency_radius)..=(player_chunk_pos.z + emergency_radius) {
                     let chunk_pos = IVec3::new(x, y, z);
                     if !loaded_chunks.contains(&chunk_pos) && !load_queue.generating.contains(&chunk_pos) {
                         let distance = ((x - player_chunk_pos.x).pow(2) + 
                                        (y - player_chunk_pos.y).pow(2) + 
                                        (z - player_chunk_pos.z).pow(2)) as f32;
                         emergency_chunks.push((chunk_pos, distance + 2.0)); // 较低优先级
                     }
                 }
             }
         }
     }
     
     emergency_chunks.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // 第一阶段：地表优先区块（可见性高的区块）
     // 智能地表检测：使用简化的异步检测算法
     let mut surface_candidates = Vec::new();
     let is_near_surface = is_near_surface_simple; // 使用简化的异步检测
     
     if is_near_surface {
         let surface_radius = if is_fast_moving { 
             (loader_config.sphere_loading_radius * 1.5) as i32 // 快速移动时扩大范围
         } else { 
             (loader_config.sphere_loading_radius * 1.2) as i32 // 稍微扩大地表搜索范围
         };
         
         // 地表区块主要在玩家Y坐标附近的几个层级
         let surface_y_min = player_chunk_pos.y - 2;
         let surface_y_max = player_chunk_pos.y + 8; // 向上多搜索一些，包含山峰
         
         for x in (player_chunk_pos.x - surface_radius)..=(player_chunk_pos.x + surface_radius) {
             for z in (player_chunk_pos.z - surface_radius)..=(player_chunk_pos.z + surface_radius) {
                 for y in surface_y_min..=surface_y_max {
                     let chunk_pos = IVec3::new(x, y, z);
                     
                     // 计算水平距离
                     let dx = (chunk_pos.x - player_chunk_pos.x) as f32;
                     let dz = (chunk_pos.z - player_chunk_pos.z) as f32;
                     let horizontal_distance = (dx * dx + dz * dz).sqrt();
                     
                     // 在地表搜索范围内且未加载
                     if horizontal_distance <= loader_config.sphere_loading_radius * 1.2 
                        && !loaded_chunks.contains(&chunk_pos) 
                        && !load_queue.generating.contains(&chunk_pos) {
                         
                         // 地表区块优先级：距离越近优先级越高，接近玩家Y坐标的优先级更高
                         let y_distance = (chunk_pos.y - player_chunk_pos.y).abs() as f32;
                         let priority = 1000.0 - horizontal_distance - y_distance * 0.5;
                         surface_candidates.push((chunk_pos, priority));
                     }
                 }
             }
         }
         
         // 按优先级排序地表候选区块
         surface_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
     }
    
    // 第二阶段：智能区块加载（地底视线优化）
    let mut sphere_candidates = Vec::new();
    
    if is_near_surface_simple {
        // 地表模式：使用原有的球形加载算法
        let sphere_radius = if is_fast_moving { 
            (loader_config.sphere_loading_radius * 1.2) as i32 
        } else { 
            loader_config.sphere_loading_radius as i32
        };
        
        for x in (player_chunk_pos.x - sphere_radius)..=(player_chunk_pos.x + sphere_radius) {
            for z in (player_chunk_pos.z - sphere_radius)..=(player_chunk_pos.z + sphere_radius) {
                for y in (player_chunk_pos.y - sphere_radius)..=(player_chunk_pos.y + sphere_radius) {
                    let chunk_pos = IVec3::new(x, y, z);
                    
                    let dx = (chunk_pos.x - player_chunk_pos.x) as f32;
                    let dy = (chunk_pos.y - player_chunk_pos.y) as f32;
                    let dz = (chunk_pos.z - player_chunk_pos.z) as f32;
                    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                    
                    if distance <= loader_config.sphere_loading_radius 
                       && !loaded_chunks.contains(&chunk_pos) 
                       && !load_queue.generating.contains(&chunk_pos)
                       && !surface_candidates.iter().any(|(pos, _)| *pos == chunk_pos) {
                        
                        let priority = 1000.0 - distance;
                        sphere_candidates.push((chunk_pos, priority));
                    }
                }
            }
        }
    } else {
          // 地底模式：使用精确视线检测算法，只加载必要的区块
          if is_deep_underground_long_time {
              // 深度地下激进模式：加载玩家周围八个方向的区块以及脚下三个区块
              let essential_chunks = vec![
                  player_chunk_pos, // 玩家当前区块
                  // 周围八个方向的区块
                  IVec3::new(player_chunk_pos.x + 1, player_chunk_pos.y, player_chunk_pos.z), // 东
                  IVec3::new(player_chunk_pos.x - 1, player_chunk_pos.y, player_chunk_pos.z), // 西
                  IVec3::new(player_chunk_pos.x, player_chunk_pos.y, player_chunk_pos.z + 1), // 南
                  IVec3::new(player_chunk_pos.x, player_chunk_pos.y, player_chunk_pos.z - 1), // 北
                  IVec3::new(player_chunk_pos.x + 1, player_chunk_pos.y, player_chunk_pos.z + 1), // 东南
                  IVec3::new(player_chunk_pos.x + 1, player_chunk_pos.y, player_chunk_pos.z - 1), // 东北
                  IVec3::new(player_chunk_pos.x - 1, player_chunk_pos.y, player_chunk_pos.z + 1), // 西南
                  IVec3::new(player_chunk_pos.x - 1, player_chunk_pos.y, player_chunk_pos.z - 1), // 西北
                  // 脚下三个区块
                  IVec3::new(player_chunk_pos.x, player_chunk_pos.y - 1, player_chunk_pos.z),
                  IVec3::new(player_chunk_pos.x, player_chunk_pos.y - 2, player_chunk_pos.z),
                  IVec3::new(player_chunk_pos.x, player_chunk_pos.y - 3, player_chunk_pos.z),
              ];
              
              for chunk_pos in essential_chunks {
                  if !loaded_chunks.contains(&chunk_pos) && !load_queue.generating.contains(&chunk_pos) {
                      sphere_candidates.push((chunk_pos, 1000.0)); // 最高优先级
                  }
              }
          } else {
              // 普通地底模式
              let underground_radius = if is_fast_moving { 3 } else { 2 }; // 进一步减少地底加载范围
              
              // 地底精确视线检测：只加载玩家视线范围内的关键区块
              for x in (player_chunk_pos.x - underground_radius)..=(player_chunk_pos.x + underground_radius) {
                  for z in (player_chunk_pos.z - underground_radius)..=(player_chunk_pos.z + underground_radius) {
                      for y in (player_chunk_pos.y - 1)..=(player_chunk_pos.y + 1) { // 地底只关注当前层和上下一层
                          let chunk_pos = IVec3::new(x, y, z);
                          
                          let dx = (chunk_pos.x - player_chunk_pos.x) as f32;
                          let dy = (chunk_pos.y - player_chunk_pos.y) as f32;
                          let dz = (chunk_pos.z - player_chunk_pos.z) as f32;
                          let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                          
                          // 地底精确视线检测：只加载最近的区块
                          if distance <= underground_radius as f32
                             && !loaded_chunks.contains(&chunk_pos) 
                             && !load_queue.generating.contains(&chunk_pos) {
                              
                              // 地底优先级：玩家当前Y层最高优先级
                              let y_penalty = if dy.abs() < 0.1 { 0.0 } else { dy.abs() * 3.0 }; // 当前Y层无惩罚
                              let priority = 1000.0 - distance - y_penalty;
                              sphere_candidates.push((chunk_pos, priority));
                          }
                      }
                  }
              }
          }
      }
    
    // 按优先级排序球形候选区块
    sphere_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    // 分配加载配额（地底模式优化）
    let mut chunks_to_add = Vec::new();
    let max_per_frame = if emergency_load { 
        if is_near_surface { 64 } else { 32 } // 地底紧急情况减少加载
    } else if is_fast_moving { 
        if is_near_surface { 48 } else { 24 } // 地底快速移动减少加载
    } else { 
        if is_near_surface { 16 } else { 8 } // 地底正常情况大幅减少加载
    };
    let mut remaining_quota = available_quota.min(max_per_frame);
    
    // 紧急加载优先
    if emergency_load {
        let emergency_to_add = emergency_chunks.len().min(10).min(remaining_quota); // 最多10个紧急区块
        for i in 0..emergency_to_add {
            chunks_to_add.push(emergency_chunks[i].0);
            remaining_quota -= 1;
        }
    }
    
    // 首先分配地表优先配额
    let surface_quota = loader_config.surface_priority_quota.min(remaining_quota);
    let surface_to_add = surface_candidates.len().min(surface_quota);
    
    for i in 0..surface_to_add {
        if !chunks_to_add.contains(&surface_candidates[i].0) {
            chunks_to_add.push(surface_candidates[i].0);
            remaining_quota -= 1;
        }
    }
    
    // 然后分配剩余配额给球形区块
    let sphere_to_add = sphere_candidates.len().min(remaining_quota);
    for i in 0..sphere_to_add {
        if !chunks_to_add.contains(&sphere_candidates[i].0) {
            chunks_to_add.push(sphere_candidates[i].0);
        }
    }
    
    // 记录添加的数量
    let added_count = chunks_to_add.len();
    
    // 添加到加载队列
    for chunk_pos in chunks_to_add {
        load_queue.pending.push_back(chunk_pos);
    }
    
    // 输出调试信息
    if is_fast_moving {
        info!("Fast movement detected! Speed optimization active. Emergency: {}, Added: {}, Total loaded: {}", 
              emergency_load, added_count, current_loaded_count);
    }
    
    if !surface_candidates.is_empty() || !sphere_candidates.is_empty() {
         if is_deep_underground_long_time {
             info!("DEEP UNDERGROUND AGGRESSIVE MODE: {} loaded (limit: 50), {} essential candidates, added {} to queue", 
                   current_loaded_count, sphere_candidates.len(), added_count);
         } else if is_near_surface {
             info!("Surface mode: {} loaded, {} surface candidates, {} sphere candidates, added {} to queue", 
                   current_loaded_count, surface_candidates.len(), sphere_candidates.len(), 
                   added_count);
         } else if use_conservative_mode {
             info!("Conservative mode (500 limit): {} loaded, {} sphere candidates, added {} to queue", 
                   current_loaded_count, sphere_candidates.len(), added_count);
         } else {
             let underground_radius = if is_fast_moving { 3 } else { 2 };
             info!("Underground vision mode (radius {}): {} loaded, {} sphere candidates, added {} to queue", 
                   underground_radius, current_loaded_count, sphere_candidates.len(), added_count);
         }
         
         // 显示深度地下计时器状态
         if all_chunks_underground {
             if let Ok(timer) = DEEP_UNDERGROUND_TIMER.lock() {
                 if let Some(start_time) = *timer {
                     let elapsed = current_time - start_time;
                     info!("Deep underground timer: {:.1}s / 30.0s", elapsed);
                 }
             }
         }
     }
}

/// 异步区块生成系统 - 启动异步生成任务（多线程）
pub fn chunk_generation_system(
    mut commands: Commands,
    mut load_queue: ResMut<ChunkLoadQueue>,
    loader_config: Res<ChunkLoaderConfig>,
    generator_config: Res<WorldGeneratorConfig>,
    registry: Res<BlockRegistry>,
    thread_pool: Res<ChunkGenerationThreadPool>,
) {
    let mut chunks_started = 0;

    // 保守的任务启动策略，避免启动过多任务导致性能问题
    // 无论线程数多少，每帧最多启动16个新任务
    let max_tasks_per_frame = 16;

    // 每帧最多启动指定数量的生成任务
    while chunks_started < max_tasks_per_frame {
        if let Some(chunk_pos) = load_queue.pending.pop_front() {
            // 标记为正在生成
            load_queue.generating.insert(chunk_pos);

            // 克隆必要的数据用于异步任务
            let config = generator_config.clone();
            let registry_clone = registry.clone();

            // 使用自定义线程池启动异步生成任务
            let task = thread_pool.pool.spawn(async move {
                let generator = WorldGenerator::new(config);
                let mut chunk = Chunk::new(chunk_pos);
                generator.generate_chunk(&mut chunk, &registry_clone);
                chunk.compute_solid_blocks();
                chunk
            });

            // 创建任务实体
            commands.spawn(ChunkGenerationTask {
                task,
                position: chunk_pos,
            });

            chunks_started += 1;
        } else {
            break;
        }
    }
}

/// 区块完成处理系统 - 处理完成的异步生成任务（主线程优化）
pub fn chunk_completion_system(
    mut commands: Commands,
    mut task_query: Query<(Entity, &mut ChunkGenerationTask)>,
    chunk_storage: Res<ChunkStorage>,
    mut load_queue: ResMut<ChunkLoadQueue>,
    thread_pool: Res<ChunkGenerationThreadPool>,
) {
    let mut completed_tasks = Vec::new();
    
    // 保守的任务处理策略，避免主线程卡顿
    // 无论线程数多少，每帧最多处理8个完成的任务
    let max_tasks_per_frame = 8;
    let mut processed_count = 0;
    
    for (entity, mut task) in task_query.iter_mut() {
        if processed_count >= max_tasks_per_frame {
            break;
        }
        
        // 使用真正的非阻塞轮询，避免主线程卡顿
        if let Some(chunk) = future::block_on(future::poll_once(&mut task.task)) {
            completed_tasks.push((entity, task.position, chunk));
            processed_count += 1;
        }
    }
    
    // 处理完成的任务
    for (entity, chunk_pos, chunk) in completed_tasks {
        let chunk_world_pos = Vec3::new(
            chunk_pos.x as f32 * 32.0,
            chunk_pos.y as f32 * 32.0,
            chunk_pos.z as f32 * 32.0,
        );

        // 生成区块实体
        let chunk_entity = commands
            .spawn((
                chunk,
                SpatialBundle {
                    transform: Transform::from_translation(chunk_world_pos),
                    ..default()
                },
            ))
            .id();

        // 存储到区块存储中
        chunk_storage.insert(chunk_pos, chunk_entity);

        // 从生成中移除
        load_queue.generating.remove(&chunk_pos);

        // 移除任务实体
        commands.entity(entity).despawn();
    }
}

/// 积极区块卸载检测系统 - 基于数量限制的智能卸载策略
pub fn chunk_unload_detection_system(
    player_query: Query<&Transform, With<FirstPersonController>>,
    loader_config: Res<ChunkLoaderConfig>,
    chunk_query: Query<(Entity, &Chunk)>,
    mut unload_queue: ResMut<ChunkUnloadQueue>,
    time: Res<Time>,
) {
    // 添加静态变量来缓存上次检查的时间和位置
    static LAST_CHECK: Mutex<Option<(f32, Vec3)>> = Mutex::new(None);
    
    // 获取玩家位置
    let player_transform = match player_query.get_single() {
        Ok(transform) => transform,
        Err(_) => return,
    };

    let player_pos = player_transform.translation;
    let player_chunk_pos = IVec3::new(
        (player_pos.x / 32.0).floor() as i32,
        (player_pos.y / 32.0).floor() as i32,
        (player_pos.z / 32.0).floor() as i32,
    );

    // 检查是否需要更新，并检测快速移动
    let current_time = time.elapsed_seconds();
    let mut should_update = false;
    let mut is_fast_moving = false;
    
    if let Ok(mut last_check) = LAST_CHECK.lock() {
        if let Some((last_time, last_world_pos)) = *last_check {
            let time_delta = current_time - last_time;
            
            // 计算移动速度
            let distance_moved = player_pos.distance(last_world_pos);
            let speed = if time_delta > 0.0 { distance_moved / time_delta } else { 0.0 };
            
            // 检测快速移动（速度超过30单位/秒，或Y轴快速下降超过10单位）
             is_fast_moving = speed > 30.0 || (player_pos.y - last_world_pos.y) < -10.0;
             
             // 根据移动状态调整检查频率
             let check_interval = if is_fast_moving { 10.0 } else { 1.0 }; // 快速移动时大幅减少卸载频率
            
            if time_delta > check_interval {
                should_update = true;
                *last_check = Some((current_time, player_pos));
            }
        } else {
            should_update = true;
            *last_check = Some((current_time, player_pos));
        }
    }
    
    if !should_update {
        return;
    }

    // 收集所有已加载的区块信息
    let mut loaded_chunks = Vec::new();
    for (entity, chunk) in chunk_query.iter() {
        // 计算区块到玩家的距离
        let dx = (chunk.coord.x - player_chunk_pos.x) as f32;
        let dy = (chunk.coord.y - player_chunk_pos.y) as f32;
        let dz = (chunk.coord.z - player_chunk_pos.z) as f32;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        
        // 计算水平距离（用于地表优先级判断）
        let horizontal_distance = (dx * dx + dz * dz).sqrt();
        
        // 判断是否为地表区块（玩家Y坐标附近）
        let is_surface = chunk.coord.y >= player_chunk_pos.y - 2 && 
                        chunk.coord.y <= player_chunk_pos.y + 8;
        
        loaded_chunks.push((entity, chunk.coord, distance, horizontal_distance, is_surface));
    }

    let current_loaded_count = loaded_chunks.len();
    
    // 获取玩家是否在地底的信息（调整检测条件）
    let is_underground = player_chunk_pos.y < 0;
    
    // 检查是否处于深度地下激进模式
    static DEEP_UNDERGROUND_TIMER: Mutex<Option<f32>> = Mutex::new(None);
    
    let surrounding_chunks = vec![
        IVec3::new(player_chunk_pos.x + 1, player_chunk_pos.y, player_chunk_pos.z),
        IVec3::new(player_chunk_pos.x - 1, player_chunk_pos.y, player_chunk_pos.z),
        IVec3::new(player_chunk_pos.x, player_chunk_pos.y, player_chunk_pos.z + 1),
        IVec3::new(player_chunk_pos.x, player_chunk_pos.y, player_chunk_pos.z - 1),
        IVec3::new(player_chunk_pos.x + 1, player_chunk_pos.y, player_chunk_pos.z + 1),
        IVec3::new(player_chunk_pos.x + 1, player_chunk_pos.y, player_chunk_pos.z - 1),
        IVec3::new(player_chunk_pos.x - 1, player_chunk_pos.y, player_chunk_pos.z + 1),
        IVec3::new(player_chunk_pos.x - 1, player_chunk_pos.y, player_chunk_pos.z - 1),
    ];
    
    let all_chunks_underground = surrounding_chunks.iter().all(|chunk_pos| chunk_pos.y < 0);
    
    let mut is_deep_underground_long_time = false;
    if let Ok(timer) = DEEP_UNDERGROUND_TIMER.lock() {
        if let Some(start_time) = *timer {
            if current_time - start_time >= 30.0 {
                is_deep_underground_long_time = true;
            }
        }
    }
    
    // 智能卸载策略：根据移动状态和地底状态调整卸载阈值
    let unload_threshold = if is_deep_underground_long_time {
        // 深度地下激进模式：立即开始激进卸载
        60 // 只保留60个区块
    } else if is_underground {
        // 地底模式更保守，因为加载的区块更少
        if is_fast_moving {
            // 地底快速移动时几乎不卸载
            loader_config.max_loaded_chunks + 200 // 允许超出200个区块才开始卸载
        } else {
            // 地底正常移动时也很保守
            loader_config.max_loaded_chunks + 100 // 允许超出100个区块才开始卸载
        }
    } else if is_fast_moving {
        // 地表快速移动时极其保守
        loader_config.max_loaded_chunks + 150 // 允许超出150个区块才开始卸载
    } else {
        // 地表正常移动时预防性卸载
        loader_config.max_loaded_chunks * 9 / 10
    };
     
     let should_unload = current_loaded_count >= unload_threshold;
    
    if !should_unload {
        return;
    }

    // 计算需要卸载的区块数量
    let target_unload_count = if is_fast_moving {
        // 快速移动时只卸载极少量区块
        (current_loaded_count / 200).max(1) // 每次只卸载0.5%或至少1个
    } else if current_loaded_count >= loader_config.max_loaded_chunks {
        // 超过限制，卸载到90%
        current_loaded_count - (loader_config.max_loaded_chunks * 9 / 10)
    } else {
        // 预防性卸载，卸载少量区块
        (current_loaded_count / 20).max(1) // 卸载5%或至少1个
    };

    // 按卸载优先级排序：
    // 1. 非地表区块优先卸载
    // 2. 距离越远优先级越高
    // 3. 地表区块中，超出地表优先范围的优先卸载
    loaded_chunks.sort_by(|a, b| {
        let (_, _, dist_a, h_dist_a, is_surface_a) = *a;
        let (_, _, dist_b, h_dist_b, is_surface_b) = *b;
        
        // 首先按是否为地表区块分类
        match (is_surface_a, is_surface_b) {
            (false, true) => std::cmp::Ordering::Less,  // 非地表优先卸载
            (true, false) => std::cmp::Ordering::Greater, // 地表保留
            _ => {
                // 同类型区块按距离排序
                if is_surface_a && is_surface_b {
                    // 地表区块：超出地表优先范围的优先卸载
                    let surface_range = loader_config.sphere_loading_radius * 1.2;
                    let out_of_surface_a = h_dist_a > surface_range;
                    let out_of_surface_b = h_dist_b > surface_range;
                    
                    match (out_of_surface_a, out_of_surface_b) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => dist_b.partial_cmp(&dist_a).unwrap_or(std::cmp::Ordering::Equal),
                    }
                } else {
                    // 非地表区块：距离越远优先级越高
                    dist_b.partial_cmp(&dist_a).unwrap_or(std::cmp::Ordering::Equal)
                }
            }
        }
    });

    // 添加到卸载队列
    let mut unloaded_count = 0;
    for (entity, coord, distance, _, _is_surface) in loaded_chunks.iter() {
        if unloaded_count >= target_unload_count {
            break;
        }
        
        // 深度地下激进模式：保护玩家周围八个方向的区块以及脚下三个区块
        if is_deep_underground_long_time {
            let essential_chunks = vec![
                player_chunk_pos, // 玩家当前区块
                // 周围八个方向的区块
                IVec3::new(player_chunk_pos.x + 1, player_chunk_pos.y, player_chunk_pos.z), // 东
                IVec3::new(player_chunk_pos.x - 1, player_chunk_pos.y, player_chunk_pos.z), // 西
                IVec3::new(player_chunk_pos.x, player_chunk_pos.y, player_chunk_pos.z + 1), // 南
                IVec3::new(player_chunk_pos.x, player_chunk_pos.y, player_chunk_pos.z - 1), // 北
                IVec3::new(player_chunk_pos.x + 1, player_chunk_pos.y, player_chunk_pos.z + 1), // 东南
                IVec3::new(player_chunk_pos.x + 1, player_chunk_pos.y, player_chunk_pos.z - 1), // 东北
                IVec3::new(player_chunk_pos.x - 1, player_chunk_pos.y, player_chunk_pos.z + 1), // 西南
                IVec3::new(player_chunk_pos.x - 1, player_chunk_pos.y, player_chunk_pos.z - 1), // 西北
                // 脚下三个区块
                IVec3::new(player_chunk_pos.x, player_chunk_pos.y - 1, player_chunk_pos.z),
                IVec3::new(player_chunk_pos.x, player_chunk_pos.y - 2, player_chunk_pos.z),
                IVec3::new(player_chunk_pos.x, player_chunk_pos.y - 3, player_chunk_pos.z),
            ];
            
            if essential_chunks.contains(coord) {
                continue; // 保护必要区块
            }
        } else {
            // 确保不卸载玩家当前所在的区块
            if *coord == player_chunk_pos {
                continue;
            }
            
            // 快速移动时大幅扩大保护范围
            let protection_radius = if is_fast_moving { 6 } else { 2 }; // 快速移动时扩大保护范围
            let dx = (coord.x - player_chunk_pos.x).abs();
            let dy = (coord.y - player_chunk_pos.y).abs();
            let dz = (coord.z - player_chunk_pos.z).abs();
            if dx <= protection_radius && dy <= protection_radius && dz <= protection_radius {
                continue;
            }
        }
        
        if !unload_queue.pending.iter().any(|(e, _)| *e == *entity) {
            unload_queue.pending.push_back((*entity, *coord));
            unloaded_count += 1;
        }
    }
    
    // 输出调试信息
    if unloaded_count > 0 {
        info!("Smart unload (fast_moving: {}): {} loaded chunks, target unload {}, actually queued {}", 
              is_fast_moving, current_loaded_count, target_unload_count, unloaded_count);
    }
}

/// 异步区块卸载系统 - 启动异步卸载任务
pub fn chunk_unload_system(
    mut commands: Commands,
    mut unload_queue: ResMut<ChunkUnloadQueue>,
    thread_pool: Res<ChunkGenerationThreadPool>,
) {
    let mut chunks_started = 0;
    let max_unload_tasks_per_frame = 5; // 每帧最多启动5个卸载任务

    // 启动异步卸载任务
    while chunks_started < max_unload_tasks_per_frame {
        if let Some((entity, chunk_pos)) = unload_queue.pending.pop_front() {
            // 标记为正在卸载
            unload_queue.unloading.insert(chunk_pos);

            // 创建异步卸载任务（在后台线程中执行清理工作）
            let task = thread_pool.pool.spawn(async move {
                // 在这里可以执行一些清理工作，比如保存区块数据等
                // 使用异步延时而不是阻塞延时
                futures_lite::future::yield_now().await;
            });

            // 创建卸载任务实体
            commands.spawn(ChunkUnloadTask {
                task,
                position: chunk_pos,
                entity,
            });

            chunks_started += 1;
        } else {
            break;
        }
    }
}

/// 区块卸载完成处理系统 - 处理完成的异步卸载任务
pub fn chunk_unload_completion_system(
    mut commands: Commands,
    mut task_query: Query<(Entity, &mut ChunkUnloadTask)>,
    chunk_query: Query<Entity, With<Chunk>>, // 添加区块查询以验证实体存在
    chunk_storage: Res<ChunkStorage>,
    mut unload_queue: ResMut<ChunkUnloadQueue>,
) {
    let mut completed_tasks = Vec::new();
    
    for (task_entity, mut unload_task) in task_query.iter_mut() {
        // 检查任务是否完成
        if let Some(_) = future::block_on(future::poll_once(&mut unload_task.task)) {
            completed_tasks.push((task_entity, unload_task.entity, unload_task.position));
        }
    }
    
    // 处理完成的卸载任务
    for (task_entity, chunk_entity, chunk_pos) in completed_tasks {
        // 安全地销毁区块实体 - 首先检查实体是否仍然存在
        if chunk_query.get(chunk_entity).is_ok() {
            // 实体存在，安全地销毁
            if let Some(entity_commands) = commands.get_entity(chunk_entity) {
                entity_commands.despawn_recursive();
                info!("Unloaded chunk at {:?}", chunk_pos);
            }
        } else {
            // 实体已经不存在，只需要清理相关数据
            warn!("Chunk entity {:?} at {:?} was already despawned", chunk_entity, chunk_pos);
        }
        
        // 从存储中移除
        chunk_storage.remove(&chunk_pos);
        
        // 从卸载中移除
        unload_queue.unloading.remove(&chunk_pos);
        
        // 移除卸载任务实体
        if let Some(mut task_entity_commands) = commands.get_entity(task_entity) {
            task_entity_commands.despawn();
        }
    }
}

/// 区块加载器插件
pub struct ChunkLoaderPlugin;

impl Plugin for ChunkLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkLoaderConfig::default())
           .insert_resource(ChunkLoadQueue::default())
           .insert_resource(ChunkUnloadQueue::default())
           .insert_resource(ChunkGenerationThreadPool::new(32)) // 默认32个线程
           .add_systems(Update, (
               thread_pool_management_system,
               chunk_demand_system,
               chunk_generation_system,
               chunk_completion_system,
               chunk_unload_detection_system,
               chunk_unload_system,
               chunk_unload_completion_system,
           ).chain().run_if(in_state(GameState::InGame))); // 使用 chain() 确保系统按顺序执行
    }
}