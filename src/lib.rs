#[cfg(test)]
#[macro_use]
extern crate proptest;

extern crate x11;

pub mod atom;
pub mod client;
pub mod draw;
pub mod input;
pub mod layouts;
pub mod output;
pub mod util;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
