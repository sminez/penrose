// Bindings with invalid modifier names are rejected
use penrose_proc::validate_user_bindings;

fn main() {
    validate_user_bindings!(("NOTAREALMODIFIER-a")());
}
