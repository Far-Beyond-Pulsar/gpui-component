//! A debug node that prints a string to the console.

/// Print a string to the console for debugging.
fn print_string() {
    println!("[DEBUG] {}", message);
}

//! A simple node that prints a message to the console.

// Print a message to the console.
fn println() {
    println!("{}", message);
}

fn main() {
    print_string("Hello World!");
    println("");

}

