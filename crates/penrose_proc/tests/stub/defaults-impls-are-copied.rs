// The stub trait should have the same default impls as the original
// NOTE: There is a limitation to the rewrite required for this to work which is that any calls
//       to methods on self inside of macro invocations are _not_ rewritten due to the way that
//       macros handle their input tokens.
use penrose_proc::stubbed_companion_trait;

#[derive(Debug, Eq, PartialEq)]
enum Animal {
    Cat,
    Dog,
}

#[stubbed_companion_trait]
trait Foo {
    #[stub("red")]
    fn color(&self, x: u32) -> &str;

    #[stub(Some(Animal::Cat))]
    fn animal(&self, name: &str) -> Option<Animal>;

    fn colored_animal(&self, name: &str, x: u32) -> String {
        if let Some(a) = self.animal(name) {
            let color = self.color(x);
            format!("A {:?} that is {}", a, color)
        } else {
            format!("Not sure what a {} is", name)
        }
    }
}

struct MyStruct {}

impl StubFoo for MyStruct {}

fn main() {
    let s = MyStruct {};
    assert_eq!(s.color(42), "red");
    assert_eq!(s.animal("anything"), Some(Animal::Cat));
    assert_eq!(&s.colored_animal("anything", 42), "A Cat that is red");
}
