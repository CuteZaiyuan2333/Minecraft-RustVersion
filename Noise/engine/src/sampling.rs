use crate::api::*;
use crate::graph::*;
use fastnoise_lite::{FastNoiseLite, NoiseType, FractalType};

pub struct SimpleEngine {
    pub graph: Graph,
    #[allow(dead_code)]
    compiled: Option<CompiledGraph>,
    seed: u64,
}

impl SimpleEngine {
    pub fn new(graph: Graph) -> Self {
        Self { graph, compiled: None, seed: 0 }
    }
}

impl NoiseEngine for SimpleEngine {
    fn validate_graph(&self) -> Result<(), NoiseError> {
        // TODO: real validation (acyclic, input arity, etc.)
        if self.graph.nodes.is_empty() { return Err(NoiseError::GraphValidation("empty graph".into())); }
        Ok(())
    }

    fn bake(&mut self, seed: Seed) { self.seed = seed.0; }

    fn sample_region(&self, req: &RegionRequest, channels: &ChannelsSpec) -> Result<RegionResult, NoiseError> {
        let mut out_channels = Vec::new();
        for ch in &channels.0 {
            match ch.kind {
                ChannelKind::Height2D | ChannelKind::Biome2D | ChannelKind::WaterLevel2D => {
                    let width = req.size[0];
                    let height = req.size[1];
                    let mut f = FastNoiseLite::with_seed(self.seed as i32);
                    f.set_noise_type(Some(NoiseType::Perlin));
                    f.set_frequency(Some(0.01));
                    if let ChannelKind::Biome2D = ch.kind { f.set_fractal_type(Some(FractalType::FBm)); }
                    let mut data = Vec::with_capacity((width * height) as usize);
                    for y in 0..height { for x in 0..width {
                        let wx = req.origin[0] as f32 + x as f32;
                        let wy = req.origin[1] as f32 + y as f32;
                        let v = f.get_noise_2d(wx, wy);
                        data.push(v);
                    }}
                    out_channels.push(ChannelData::Scalar2D { name: ch.name.clone(), width, height, data });
                }
                _ => {
                    let width = req.size[0];
                    let height = req.size[1];
                    let depth = req.size[2];
                    let mut f = FastNoiseLite::with_seed(self.seed as i32);
                    f.set_noise_type(Some(NoiseType::OpenSimplex2));
                    f.set_frequency(Some(0.02));
                    let mut data = Vec::with_capacity((width * height * depth) as usize);
                    for z in 0..depth { for y in 0..height { for x in 0..width {
                        let wx = req.origin[0] as f32 + x as f32;
                        let wy = req.origin[1] as f32 + y as f32;
                        let wz = req.origin[2] as f32 + z as f32;
                        let v = f.get_noise_3d(wx, wy, wz);
                        data.push(v);
                    }}}
                    out_channels.push(ChannelData::Scalar3D { name: ch.name.clone(), width, height, depth, data });
                }
            }
        }
        Ok(RegionResult { origin: req.origin, size: req.size, channels: out_channels })
    }
}