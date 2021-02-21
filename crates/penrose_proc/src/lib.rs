//! Proc macros for use in the main Penrose crate
#![warn(
    broken_intra_doc_links,
    clippy::all,
    future_incompatible,
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms
)]

use proc_macro::TokenStream;

mod stub;
mod validate_bindings;

use stub::stubbed_companion_trait_inner;
use validate_bindings::validate_user_bindings_inner;

/// This is an internal macro that is used as part of `gen_keybindings` to validate user provided
/// key bindings at compile time using xmodmap.
///
/// It is not intended for use outside of that context and may be modified and updated without
/// announcing breaking API changes.
///
/// ```no_run
/// # use penrose_proc::validate_user_bindings;
/// validate_user_bindings!(
///     ( "M-a", "M-b" )
///     (
///         ( ( "M-{}", "M-S-{}" ) ( "1", "2", "3" ) )
///     )
/// );
/// ```
#[proc_macro]
pub fn validate_user_bindings(input: TokenStream) -> TokenStream {
    validate_user_bindings_inner(input)
}

/// Auto derive a dummy implementation of the marked trait for use in test cases.
///
/// This is _not_ a full blown mocking implemention, it simply allows for quickly setting default
/// return values on trait methods you don't need for a given test case and allows for easily
/// testing functionality that only makes use of a handful of methods.
///
/// Wherever possible, you should always prefer breaking traits down into small, composible units
/// instead of "mega-traits" but in the case of `XConn` and some of its component parts, further
/// granularity isn't really desirable.
#[proc_macro_attribute]
pub fn stubbed_companion_trait(args: TokenStream, input: TokenStream) -> TokenStream {
    stubbed_companion_trait_inner(args, input)
}
