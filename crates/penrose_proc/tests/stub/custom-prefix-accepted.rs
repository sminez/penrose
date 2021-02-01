// Custom prefixes are accepted and used in place of "Stub"
use penrose_proc::stubbed_companion_trait;

#[allow(dead_code)]
#[derive(Eq, PartialEq, Debug)]
enum Scale {
    Major,
    Minor,
    Blues,
}

// Should generated CustomPrefixBar
#[stubbed_companion_trait(prefix = "CustomPrefix")]
trait Bar {
    #[stub("F#")]
    fn musical_note(&self, x: u32) -> &str;

    #[stub(Scale::Blues)]
    fn musical_scale(&self, f: f64) -> Scale;
}

struct MyStruct {}

impl CustomPrefixBar for MyStruct {}

fn main() {
    let s = MyStruct {};
    assert_eq!(s.musical_note(17), "F#");
    assert_eq!(s.musical_scale(3.14), Scale::Blues);
}
