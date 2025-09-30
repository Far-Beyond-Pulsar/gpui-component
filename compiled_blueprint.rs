//! A node that prints a string to the console for debugging.
//!
//! This node outputs the provided string to the console, prefixed with "[DEBUG]".
//! Useful for inspecting values, debugging logic, or tracing execution in your graph.
//!
//! # Inputs
//! - `in_message_string`: The string message to print.
//!
//! # Outputs
//! - None. The message is printed to the console.
//!
//! # Example
//! If `in_message_string` is "Hello, world!", the output will be:
//! [DEBUG] Hello, world!
//!
//! # Notes
//! Use this node for development and troubleshooting. Remove or disable in production code.

/// Print a string to the console for debugging.
fn print_string() {
    println!("[DEBUG] {}", message);
}

//! # Thread Spawn Node
//!
//! A node that spawns a new thread to execute connected code concurrently.
//!
//! This node creates a new operating system thread and executes the code
//! connected to its body in that thread. The new thread runs concurrently
//! with the current thread, allowing for parallel execution.
//!
//! # Outputs
//! - `std::thread::JoinHandle<()>`: A handle to the spawned thread that can
//!   be used to wait for the thread to complete or check its status
//!
//! # Execution Outputs
//! - `continue`: Executes immediately after spawning the thread
//! - `body`: Executes in the spawned thread
//!
//! # Behavior
//! The spawned thread will execute the code in `pulsar_exec_body` and
//! terminate when that code completes. The main thread continues execution
//! immediately after spawning.
//!
//! # Example
//! Use this node to perform background work, such as file I/O or computation,
//! without blocking the main thread. You can join the thread later if you
//! need to wait for its completion.
//!
//! # Notes
//! - Thread spawning has overhead. Use for tasks that benefit from parallelism.
//! - Remember to join threads if you need to wait for their completion.
//! - This node creates a new operating system thread and executes the code
//! connected to its body in that thread. The new thread runs concurrently
//! with the current thread, allowing for parallel execution.

/// Spawn a new thread to execute code.
fn thread_spawn() -> std::thread::JoinHandle<()> {
    let handle = std::thread::spawn(|| {
        {}
    });
    {}
    handle
}

//! A node that parks (suspends) the current thread until it is unparked.
//!
//! This node suspends the current thread, putting it to sleep until another thread
//! explicitly unparks it. Useful for implementing custom synchronization primitives
//! or waiting for external events.
//!
//! # Inputs
//! - None
//!
//! # Outputs
//! - None (the thread is parked and resumes only when unparked)
//!
//! # Example
//! Use this node in conjunction with thread unpark logic to coordinate thread execution.
//!
//! # Notes
//! - The thread will remain parked indefinitely until explicitly unparked.
//! - If the thread is already unparked when `park()` is called, it will return immediately.
//! - Use with caution to avoid deadlocks.

/// Park the current thread until unparked.
fn thread_park() {
    std::thread::park();
}

fn main() {
    print_string("Hello World!");
}

