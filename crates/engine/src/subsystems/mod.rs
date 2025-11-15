//! Frontend Subsystems
//!
//! These subsystems coordinate frontend operations and interface with backend:
//! - Window management: Creating, destroying, and managing window lifecycle
//! - Task management: Spawning and coordinating async tasks with UI

pub mod task_mgr;
pub mod window_mgr;

pub use task_mgr::TaskManager;
pub use window_mgr::{WindowManager, WindowRequest};
