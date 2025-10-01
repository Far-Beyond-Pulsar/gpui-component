// Auto-generated code from Pulsar Blueprint
// DO NOT EDIT - Changes will be overwritten

use pulsar_std::*;

pub fn begin_play() {
    print_string(Default::default());
    if greater_than (add (2 , 3) , 3) { print_string (Default :: default ()) ; } else { }
}

pub fn main() {
    if greater_than (add (2 , 3) , 3) { print_string ("Result is greater than 3! \u{2713}") ; } else { print_string ("Result is 3 or less. \u{2717}") ; }
}

