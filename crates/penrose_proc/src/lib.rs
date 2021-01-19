use proc_macro::TokenStream;
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    LitStr, Token,
};

use std::{collections::HashSet, process::Command};

const VALID_PREFIXES: [&str; 4] = ["A", "M", "S", "C"];

struct BindingsInput {
    bindings: Vec<String>,
}

impl Parse for BindingsInput {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            bindings: Punctuated::<LitStr, Token![,]>::parse_terminated(&input)?
                .iter()
                .map(LitStr::value)
                .collect(),
        })
    }
}

fn keynames_from_xmodmap() -> Vec<String> {
    let res = Command::new("xmodmap")
        .arg("-pke")
        .output()
        .expect("unable to fetch keycodes via xmodmap: please ensure that it is installed");

    // each line should match 'keycode <code> = <names ...>'
    String::from_utf8(res.stdout)
        .expect("received invalid utf8 from xmodmap")
        .lines()
        .flat_map(|s| s.split_whitespace().skip(3).map(|name| name.into()))
        .collect()
}

fn is_valid_binding(pattern: &str, names: &[String]) -> bool {
    let mut parts: Vec<&str> = pattern.split('-').collect();
    if let Some(s) = parts.pop() {
        if names.contains(&s.to_string()) {
            return parts.iter().all(|s| VALID_PREFIXES.contains(s));
        }
    }

    false
}

#[proc_macro]
pub fn validate_user_bindings(input: TokenStream) -> TokenStream {
    let BindingsInput { bindings } = parse_macro_input!(input as BindingsInput);
    let names = keynames_from_xmodmap();
    let mut seen = HashSet::new();

    for b in bindings.iter() {
        if seen.contains(b) {
            panic!("'{}' is bound as a keybinding more than once", b);
        } else {
            seen.insert(b);
        }

        if !is_valid_binding(b, &names) {
            panic!(
                "'{}' is an invalid keybinding: keybindings should be of the form <modifiers>-<key name> \
                 with modifiers being any of {:?}. (Key names can be obtained by running 'xmodmap -pke' \
                 in a terminal) \ne.g:  M-j, M-S-slash, M-C-Up",
                b, VALID_PREFIXES
            )
        }
    }

    // If everything is fine then just consume the input
    TokenStream::new()
}
