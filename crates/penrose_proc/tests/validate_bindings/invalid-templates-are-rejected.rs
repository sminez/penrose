// Bindings that use valid modifiers and valid keys in templates are accepted
use penrose_proc::validate_user_bindings;

fn main() {
    validate_user_bindings!(()((("Not a template")("1", "2", "3"))));
}
