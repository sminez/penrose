// Repeated raw bindings are rejected
use penrose_proc::validate_user_bindings;

fn main() {
    validate_user_bindings!(("M-a", "M-a")());
}
