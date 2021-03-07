// Bindings that use valid modifiers and valid keys are accepted
use penrose_proc::validate_user_bindings;

fn main() {
    validate_user_bindings!((
        "M-a",
        "A-b",
        "S-c",
        "C-d",
        "M-A-e",
        "A-S-f",
        "M-S-C-g",
        "M-A-S-C-h"
    )());
}
