pub mod engine_main;
pub mod classes;

fn main() {
    engine_main::main();
    classes::ExampleClass::events::main::main();
}
