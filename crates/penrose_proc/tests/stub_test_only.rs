use penrose_proc::stubbed_companion_trait;

#[allow(dead_code)]
#[derive(Debug, Eq, PartialEq)]
enum Animal {
    Cat,
    Dog,
}

// Should be usable in the test case below
#[stubbed_companion_trait(test_only = "true")]
trait Foo {
    #[stub("red")]
    fn color(&self, x: u32) -> &str;
}

#[stubbed_companion_trait(test_only = "true", prefix = "Custom")]
trait Bar: Foo {
    /// the animal method should have this comment
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
struct MyStruct;

impl StubFoo for MyStruct {}
impl CustomBar for MyStruct {}

#[test]
fn can_use_stubs_in_tests() {
    let s = MyStruct;
    assert_eq!(s.color(42), "red");
}
