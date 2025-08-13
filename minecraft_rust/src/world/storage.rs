use bevy::prelude::*;
use dashmap::DashMap;

#[derive(Resource, Default)]
pub struct ChunkStorage {
    pub chunks: DashMap<IVec3, Entity>,
}

impl ChunkStorage {
    pub fn new() -> Self {
        Self {
            chunks: DashMap::new(),
        }
    }

    pub fn insert(&self, coord: IVec3, entity: Entity) { 
        self.chunks.insert(coord, entity); 
    }
    
    pub fn get(&self, coord: &IVec3) -> Option<Entity> { 
        self.chunks.get(coord).map(|e| *e.value()) 
    }

    pub fn remove(&self, coord: &IVec3) -> Option<Entity> {
        self.chunks.remove(coord).map(|(_, entity)| entity)
    }
}