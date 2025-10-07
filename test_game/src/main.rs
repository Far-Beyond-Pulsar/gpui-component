pub mod engine_main;
pub mod classes;

fn main() {
    engine_main::main();
    classes::ExampleClass::events::begin_play::begin_play();
}