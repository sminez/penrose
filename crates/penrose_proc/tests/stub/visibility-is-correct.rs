// The stub trait has the same visibility as the parent
mod inner {
    use penrose_proc::stubbed_companion_trait;

    #[stubbed_companion_trait]
    pub trait Foo {
        #[stub("red")]
        fn color(&self, x: u32) -> &str;
    }
}

use inner::Foo;

struct MyStruct {}
impl inner::StubFoo for MyStruct {}

fn main() {
    let s = MyStruct {};
    assert_eq!(s.color(42), "red");
}
