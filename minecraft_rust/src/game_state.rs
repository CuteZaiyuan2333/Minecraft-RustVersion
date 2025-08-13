use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;

/// 游戏状态枚举
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    InGame,
    Paused,
}

/// 世界存档信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldInfo {
    pub name: String,
    pub seed: u32,
    pub created_time: String,
    pub last_played: String,
    pub game_mode: GameMode,
    pub world_type: WorldType,
}

impl Default for WorldInfo {
    fn default() -> Self {
        Self {
            name: "新世界".to_string(),
            seed: 12345,
            created_time: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            last_played: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            game_mode: GameMode::Creative,
            world_type: WorldType::Default,
        }
    }
}

/// 游戏模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum GameMode {
    Survival,
    #[default]
    Creative,
    Adventure,
    Spectator,
}

impl GameMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            GameMode::Survival => "生存模式",
            GameMode::Creative => "创造模式",
            GameMode::Adventure => "冒险模式",
            GameMode::Spectator => "旁观模式",
        }
    }
}

/// 世界类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum WorldType {
    #[default]
    Default,
    Flat,
    LargeBiomes,
    Amplified,
}

impl WorldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorldType::Default => "默认",
            WorldType::Flat => "超平坦",
            WorldType::LargeBiomes => "巨型生物群系",
            WorldType::Amplified => "放大化",
        }
    }
}

/// 异步保存任务
#[derive(Component)]
pub struct SaveTask {
    pub task: Task<Result<(), String>>,
}

/// 保存队列 - 避免重复保存同一个世界
#[derive(Resource, Default)]
pub struct SaveQueue {
    pub pending_saves: HashMap<String, String>, // world_name -> last_played_time
}

/// 保存任务检查定时器 - 限制检查频率以减少IO
#[derive(Resource)]
pub struct SaveTaskTimer {
    pub timer: Timer,
}

impl Default for SaveTaskTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating), // 每秒检查一次
        }
    }
}

/// 世界管理器
#[derive(Resource, Default)]
pub struct WorldManager {
    pub worlds: HashMap<String, WorldInfo>,
    pub current_world: Option<String>,
    pub saves_directory: PathBuf,
}

impl WorldManager {
    pub fn new() -> Self {
        let saves_dir = PathBuf::from("saves");
        if !saves_dir.exists() {
            if let Err(e) = fs::create_dir_all(&saves_dir) {
                error!("Failed to create saves directory: {}", e);
            }
        }

        let mut manager = Self {
            worlds: HashMap::new(),
            current_world: None,
            saves_directory: saves_dir,
        };

        manager.load_worlds();
        manager
    }

    /// 加载所有世界存档
    pub fn load_worlds(&mut self) {
        self.worlds.clear();
        
        if let Ok(entries) = fs::read_dir(&self.saves_directory) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let world_name = entry.file_name().to_string_lossy().to_string();
                    let world_info_path = entry.path().join("world_info.json");
                    
                    if world_info_path.exists() {
                        match fs::read_to_string(&world_info_path) {
                            Ok(content) => {
                                match serde_json::from_str::<WorldInfo>(&content) {
                                    Ok(world_info) => {
                                        self.worlds.insert(world_name, world_info);
                                    }
                                    Err(e) => {
                                        warn!("Failed to parse world info for {}: {}", world_name, e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to read world info for {}: {}", world_name, e);
                            }
                        }
                    }
                }
            }
        }
        
        info!("Loaded {} world saves", self.worlds.len());
    }

    /// 创建新世界
    pub fn create_world(&mut self, world_info: WorldInfo) -> Result<(), Box<dyn std::error::Error>> {
        let world_dir = self.saves_directory.join(&world_info.name);
        
        // 检查世界是否已存在
        if world_dir.exists() {
            return Err("世界已存在".into());
        }

        // 创建世界目录
        fs::create_dir_all(&world_dir)?;

        // 保存世界信息
        let world_info_path = world_dir.join("world_info.json");
        let world_info_json = serde_json::to_string_pretty(&world_info)?;
        fs::write(world_info_path, world_info_json)?;

        // 添加到世界列表
        self.worlds.insert(world_info.name.clone(), world_info);
        
        info!("Created new world: {}", self.worlds.len());
        Ok(())
    }

    /// 删除世界
    pub fn delete_world(&mut self, world_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(_) = self.worlds.remove(world_name) {
            let world_dir = self.saves_directory.join(world_name);
            if world_dir.exists() {
                fs::remove_dir_all(world_dir)?;
            }
            info!("Deleted world: {}", world_name);
        }
        Ok(())
    }

    /// 选择当前世界
    pub fn select_world(&mut self, world_name: String) {
        if self.worlds.contains_key(&world_name) {
            self.current_world = Some(world_name);
        }
    }

    /// 获取当前世界信息
    pub fn get_current_world(&self) -> Option<&WorldInfo> {
        self.current_world.as_ref().and_then(|name| self.worlds.get(name))
    }

    /// 更新世界最后游玩时间（仅更新内存，不立即保存）
    pub fn update_last_played(&mut self, world_name: &str) {
        if let Some(world_info) = self.worlds.get_mut(world_name) {
            world_info.last_played = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        }
    }
    
    /// 异步保存世界信息
    pub fn save_world_info_async(&self, world_name: &str, commands: &mut Commands, save_queue: &mut SaveQueue) {
        if let Some(world_info) = self.worlds.get(world_name) {
            let current_time = world_info.last_played.clone();
            
            // 检查是否已经有相同的保存任务在队列中
            if let Some(pending_time) = save_queue.pending_saves.get(world_name) {
                if pending_time == &current_time {
                    return; // 已经有相同的保存任务，跳过
                }
            }
            
            // 添加到保存队列
            save_queue.pending_saves.insert(world_name.to_string(), current_time);
            
            let world_info_clone = world_info.clone();
            let world_name_clone = world_name.to_string();
            let saves_directory = self.saves_directory.clone();
            
            let task_pool = AsyncComputeTaskPool::get();
            let task = task_pool.spawn(async move {
                let world_dir = saves_directory.join(&world_name_clone);
                let info_file = world_dir.join("world_info.json");
                
                match serde_json::to_string_pretty(&world_info_clone) {
                    Ok(json) => {
                        match std::fs::write(&info_file, json) {
                            Ok(_) => Ok(()),
                            Err(e) => Err(format!("Failed to write world info file: {}", e)),
                        }
                    }
                    Err(e) => Err(format!("Failed to serialize world info: {}", e)),
                }
            });
            
            commands.spawn(SaveTask { task });
        }
    }
}

/// 游戏状态管理插件
pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>()
           .init_resource::<WorldManager>()
           .init_resource::<SaveQueue>()
           .init_resource::<SaveTaskTimer>()
           .add_systems(Startup, setup_world_manager)
           .add_systems(OnEnter(GameState::InGame), update_world_last_played)
           .add_systems(Update, handle_save_tasks);
    }
}

/// 设置世界管理器
fn setup_world_manager(mut world_manager: ResMut<WorldManager>) {
    world_manager.load_worlds();
}

/// 更新当前世界的最后游玩时间
fn update_world_last_played(
    mut world_manager: ResMut<WorldManager>,
    mut commands: Commands,
    mut save_queue: ResMut<SaveQueue>,
) {
    if let Some(current_world) = world_manager.current_world.clone() {
        world_manager.update_last_played(&current_world);
        world_manager.save_world_info_async(&current_world, &mut commands, &mut save_queue);
        info!("Updated last played time for world: {}", current_world);
    }
}

/// 处理异步保存任务
fn handle_save_tasks(
    time: Res<Time>,
    mut commands: Commands,
    mut save_tasks: Query<(Entity, &mut SaveTask)>,
    mut save_queue: ResMut<SaveQueue>,
    mut save_timer: ResMut<SaveTaskTimer>,
) {
    // 更新定时器
    save_timer.timer.tick(time.delta());
    
    // 只有定时器触发时才检查保存任务
    if !save_timer.timer.just_finished() {
        return;
    }
    
    for (entity, mut save_task) in &mut save_tasks {
        if let Some(result) = future::block_on(future::poll_once(&mut save_task.task)) {
            match result {
                Ok(_) => {
                    debug!("World info saved successfully");
                }
                Err(e) => {
                    error!("Failed to save world info: {}", e);
                }
            }
            
            // 清理完成的任务
            commands.entity(entity).despawn();
        }
    }
    
    // 定期清理保存队列中的旧条目（避免内存泄漏）
    if save_queue.pending_saves.len() > 100 {
        save_queue.pending_saves.clear();
    }
}

/// 处理ESC键切换暂停状态
fn handle_escape_key(
    keyboard: Res<Input<KeyCode>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut windows: Query<&mut Window>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        match current_state.get() {
            GameState::InGame => {
                next_state.set(GameState::Paused);
                // 解锁鼠标
                if let Ok(mut window) = windows.get_single_mut() {
                    window.cursor.grab_mode = bevy::window::CursorGrabMode::None;
                    window.cursor.visible = true;
                }
            }
            GameState::Paused => {
                next_state.set(GameState::InGame);
                // 锁定鼠标
                if let Ok(mut window) = windows.get_single_mut() {
                    window.cursor.grab_mode = bevy::window::CursorGrabMode::Confined;
                    window.cursor.visible = false;
                }
            }
            _ => {}
        }
    }
}