// Bindings that use valid modifiers and valid keys are accepted
use penrose_proc::validate_user_bindings;

fn main() {
    validate_user_bindings!((
        "a",
        "M-b",
        "A-c",
        "S-d",
        "C-e",
        "M-A-f",
        "A-S-g",
        "M-S-C-h",
        "M-A-S-C-i",
    )());
}
