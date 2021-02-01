// The stub trait is generated and we can implement it as normal
use penrose_proc::stubbed_companion_trait;

// Should generated StubFoo
#[stubbed_companion_trait]
trait Foo {
    #[stub("red")]
    fn color(&self, x: u32) -> &str;
}

struct MyStruct;
impl StubFoo for MyStruct {}

fn main() {
    let s = MyStruct;
    assert_eq!(s.color(42), "red");
}
