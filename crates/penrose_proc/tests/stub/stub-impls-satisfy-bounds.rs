// The stub trait is generated with the same supertraits
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
}

#[stubbed_companion_trait]
trait Bar: Foo {
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

#[derive(Debug)]
struct MyStruct {}

impl StubFoo for MyStruct {}
impl StubBar for MyStruct {}

fn main() {
    let s = MyStruct {};
    assert_eq!(&s.colored_animal("anything", 42), "A Cat that is red");
}
