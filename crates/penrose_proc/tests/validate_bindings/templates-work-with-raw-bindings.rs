// Bindings that use valid modifiers and valid keys in templates are accepted
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
        "M-A-S-C-i"
    )(((
        "{}",
        "M-{}",
        "M-S-{}",
        "M-C-{}",
        "M-S-C-{}",
        "M-A-C-{}",
        "M-S-C-A-{}"
    )("1", "2", "3"))((
        "{}",
        "M-{}",
        "M-S-{}",
        "M-C-{}",
        "M-S-C-{}",
        "M-A-C-{}",
        "M-S-C-A-{}"
    )("Return", "Tab"))));
}
