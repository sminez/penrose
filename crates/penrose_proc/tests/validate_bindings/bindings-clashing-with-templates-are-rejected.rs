// Bindings that use valid modifiers and valid keys are accepted
use penrose_proc::validate_user_bindings;

fn main() {
    validate_user_bindings!(("M-a")((("M-{}")("a"))));
}
