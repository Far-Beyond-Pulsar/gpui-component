//! A debug node that prints a string to the console.

/// Print a string to the console for debugging.
fn print_string() {
    println!("[DEBUG] {}", message);
}

fn main() {
    print_string("Hello World!");
    print_string("");

}

