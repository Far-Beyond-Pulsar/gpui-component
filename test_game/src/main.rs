pub mod engine_main;
pub mod classes;
pub use std::fs::

fn main() {
    engine_main::main();
    classes::ExampleClass::events::begin_play();
}