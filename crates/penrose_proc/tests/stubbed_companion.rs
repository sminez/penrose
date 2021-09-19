use penrose_proc::stubbed_companion_trait;

#[derive(Debug, Eq, PartialEq)]
enum Animal {
    Cat,
    Dog,
}

#[stubbed_companion_trait]
trait Colors {
    #[stub("red")]
    fn color(&self, x: u32) -> &str;
}

#[stubbed_companion_trait]
trait AnimalsAndColors: Colors {
    #[stub(Some(Animal::Cat))]
    fn animal(&self, name: &str) -> Option<Animal>;

    fn best_animal(&self) -> Animal {
        Animal::Dog
    }

    fn colored_animal(&self, name: &str, x: u32) -> String {
        if let Some(a) = self.animal(name) {
            let color = self.color(x);
            format!("A {:?} that is {}", a, color)
        } else {
            format!("Not sure what a {} is", name)
        }
    }
}

struct ImplColors;

impl Colors for ImplColors {
    fn color(&self, x: u32) -> &str {
        match x {
            42 => "red",
            _ => "green",
        }
    }
}

#[test]
fn it_is_possible_to_impl_the_original_trait() {
    let ic = ImplColors;
    assert_eq!(ic.color(42), "red");
    assert_eq!(ic.color(19), "green");
}

struct DeriveStubbedAnimalsAndColors;

impl StubColors for DeriveStubbedAnimalsAndColors {}
impl StubAnimalsAndColors for DeriveStubbedAnimalsAndColors {}

#[test]
fn default_method_impls_are_copied() {
    let d = DeriveStubbedAnimalsAndColors;
    assert_eq!(d.best_animal(), Animal::Dog);
}

#[test]
fn trait_bounds_are_maintained() {
    let d = DeriveStubbedAnimalsAndColors;
    assert_eq!(&d.colored_animal("anything", 42), "A Cat that is red");
}

// -----

mod inner {
    #[penrose_proc::stubbed_companion_trait]
    pub trait InnerColors {
        #[stub("blue")]
        fn color(&self, x: u32) -> &str;
    }
}

use inner::InnerColors;
struct OuterImplementer;
impl inner::StubInnerColors for OuterImplementer {}

#[test]
fn visibility_of_stubs_matches_original() {
    let o = OuterImplementer;
    assert_eq!(o.color(42), "blue");
}

// -----

#[allow(dead_code)]
#[derive(Eq, PartialEq, Debug)]
enum Scale {
    Major,
    Minor,
    Blues,
}

#[stubbed_companion_trait(prefix = "CustomPrefix")]
trait Musical {
    #[stub("F#")]
    fn musical_note(&self, x: u32) -> &str;

    #[stub(Scale::Blues)]
    fn musical_scale(&self, f: f64) -> Scale;
}

struct ImplMusical;
impl CustomPrefixMusical for ImplMusical {}

#[test]
fn custom_prefix_works() {
    let i = ImplMusical;
    assert_eq!(i.musical_note(17), "F#");
    assert_eq!(i.musical_scale(3.14), Scale::Blues);
}

// -----
