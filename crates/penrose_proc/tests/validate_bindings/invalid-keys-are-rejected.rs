// Bindings with invalid key names are rejected
use penrose_proc::validate_user_bindings;

fn main() {
    validate_user_bindings!(("notarealkey")());
}
