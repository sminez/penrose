// The stub trait has the same visibility as the parent
mod inner {
    use penrose_proc::stubbed_companion_trait;

    #[stubbed_companion_trait]
    trait Foo {
        #[stub("red")]
        fn color(&self, x: u32) -> &str;
    }
}

struct MyStruct {}
// Should fail due to being private
impl inner::StubFoo for MyStruct {}

fn main() {}
