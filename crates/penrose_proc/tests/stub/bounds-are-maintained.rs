// The stub trait is generated with the same supertraits
use penrose_proc::stubbed_companion_trait;

#[stubbed_companion_trait]
trait Foo: std::fmt::Debug + Eq {
    #[stub("red")]
    fn color(&self, x: u32) -> &str;
}

#[derive(Debug)]
struct MyStruct;

impl StubFoo for MyStruct {}

fn main() {
    let s = MyStruct;
    assert_eq!(s.color(42), "red");
}
