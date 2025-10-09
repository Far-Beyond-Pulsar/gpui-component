//! Mutex Nodes
//!
//! Nodes for synchronization and safe shared state in Pulsar blueprints.
//!
//! # Node Category: Mutex
//!
//! Provides primitives for mutual exclusion and locking.

use std::sync::{Arc, Mutex, MutexGuard};
use crate::blueprint;

/// Create a new mutex wrapping a value.
///
/// # Inputs
/// - `value`: The value to protect with a mutex
///
/// # Returns
/// An `Arc<Mutex<T>>` for shared ownership and locking.
///
/// # Mutex Create
/// Creates a new mutex-protected value.
#[blueprint(type: crate::NodeTypes::pure, category: "Mutex")]
pub fn create_mutex<T>(value: T) -> Arc<Mutex<T>>
where
    T: Send + 'static,
{
    Arc::new(Mutex::new(value))
}

/// Lock a mutex for exclusive access.
///
/// # Inputs
/// - `mutex`: The mutex to lock
///
/// # Returns
/// A guard that allows access to the value.
///
/// # Mutex Lock
/// Locks the mutex and returns a guard for access.
#[blueprint(type: crate::NodeTypes::fn_, category: "Mutex")]
pub fn lock_mutex<T>(mutex: Arc<Mutex<T>>) -> MutexGuard<'static, T>
where
    T: Send + 'static,
{
    // This is a simplification; real lifetimes are more complex.
    // In blueprints, this would be managed by the runtime.
    unsafe { std::mem::transmute::<MutexGuard<'_, T>, MutexGuard<'static, T>>(mutex.lock().unwrap()) }
}

/// Unlock a mutex (drops the guard).
///
/// # Inputs
/// - `guard`: The mutex guard to drop
///
/// # Mutex Unlock
/// Unlocks the mutex by dropping the guard.
#[blueprint(type: crate::NodeTypes::fn_, category: "Mutex")]
pub fn unlock_mutex<T>(_guard: MutexGuard<'static, T>) {
    // Dropping the guard unlocks the mutex
}
