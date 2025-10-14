//! # Pulsar Engine Backend
//! 
//! This crate provides the backend functionalities for the Pulsar game engine, including
//! rendering, asset management, and core engine systems.
//! It is designed to be modular and extensible, allowing developers to
//! build high-performance games with ease.

pub mod subsystems;
pub use tokio;
pub use subsystems::physics::PhysicsEngine;
pub use std::sync::Arc;

pub const ENGINE_THREADS: [&str; 8] = [
    "GameThread",
    "RenderThread",
    "AssetLoaderThread",
    "PhysicsThread",
    "AIThread",
    "AudioThread",
    "NetworkThread",
    "InputThread",
];


pub struct EngineBackend {
    physics_engine: Arc<PhysicsEngine>,
}

impl EngineBackend {
    pub async fn init() -> Self {
        let physics_engine = Arc::new(PhysicsEngine::new());

        let physics_engine_clone = Arc::clone(&physics_engine);
        tokio::spawn(async move {
            physics_engine_clone.start().await;
        });

        EngineBackend { physics_engine }
    }
}


