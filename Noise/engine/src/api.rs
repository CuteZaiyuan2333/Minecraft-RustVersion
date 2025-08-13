use serde::{Deserialize, Serialize}; use thiserror::Error;
#[derive(Debug, Error)] pub enum NoiseError { #[error("Graph validation failed: {0}")] GraphValidation(String), #[error("Sampling error: {0}")] Sampling(String) }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct Seed(pub u64);
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct RegionRequest { pub origin: [i32; 3], pub size: [u32; 3], pub lod: u8 }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct ChannelDesc { pub name: String, pub kind: ChannelKind }
#[derive(Debug, Clone, Serialize, Deserialize)] pub enum ChannelKind { Height2D, Biome2D, Cave3D, Ore3D, WaterLevel2D, StructureMask3D }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct ChannelsSpec(pub Vec<ChannelDesc>);
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct RegionResult { pub origin: [i32; 3], pub size: [u32; 3], pub channels: Vec<ChannelData> }
#[derive(Debug, Clone, Serialize, Deserialize)] pub enum ChannelData { Scalar2D { name: String, width: u32, height: u32, data: Vec<f32> }, Scalar3D { name: String, width: u32, height: u32, depth: u32, data: Vec<f32> } }
pub trait NoiseEngine: Send + Sync { fn validate_graph(&self) -> Result<(), NoiseError>; fn bake(&mut self, seed: Seed); fn sample_region(&self, req: &RegionRequest, channels: &ChannelsSpec) -> Result<RegionResult, NoiseError>; }
