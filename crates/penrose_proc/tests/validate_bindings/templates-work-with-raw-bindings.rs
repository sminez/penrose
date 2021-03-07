// Bindings that use valid modifiers and valid keys in templates are accepted
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
    )(((
        "M-{}",
        "M-S-{}",
        "M-C-{}",
        "M-S-C-{}",
        "M-A-C-{}",
        "M-S-C-A-{}"
    )("1", "2", "3"))((
        "M-{}",
        "M-S-{}",
        "M-C-{}",
        "M-S-C-{}",
        "M-A-C-{}",
        "M-S-C-A-{}"
    )("Return", "Tab"))));
}
