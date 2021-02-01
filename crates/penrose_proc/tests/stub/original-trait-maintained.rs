// The original trait is unchanged and we can implement it as normal
use penrose_proc::stubbed_companion_trait;

#[stubbed_companion_trait]
trait Foo {
    #[stub("red")]
    fn color(&self, x: u32) -> &str;
}

struct MyStruct;

impl Foo for MyStruct {
    fn color(&self, x: u32) -> &str {
        match x {
            42 => "red",
            _ => "green",
        }
    }
}

fn main() {
    let s = MyStruct;
    assert_eq!(s.color(42), "red");
    assert_eq!(s.color(19), "green");
}
