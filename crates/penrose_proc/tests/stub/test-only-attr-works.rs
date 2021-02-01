// The stub trait is only visible in tests when the arg is passed
use penrose_proc::stubbed_companion_trait;

#[stubbed_companion_trait(test_only = "true")]
pub trait Foo {
    #[stub("red")]
    fn color(&self, x: u32) -> &str;
}

struct MyStruct;
impl StubFoo for MyStruct {}

fn main() {}
