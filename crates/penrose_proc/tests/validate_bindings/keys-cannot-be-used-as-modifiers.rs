// Bindings with using key names as modifiers are rejected
use penrose_proc::validate_user_bindings;

fn main() {
    validate_user_bindings!(("Return-a")());
}
