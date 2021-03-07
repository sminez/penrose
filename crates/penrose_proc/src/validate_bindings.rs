//! Compile time validation for user keybindings
use penrose_keysyms::XKeySym;
use proc_macro::TokenStream;
use strum::IntoEnumIterator;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    LitStr, Token,
};

use std::collections::HashSet;

const VALID_MODIFIERS: [&str; 4] = ["A", "M", "S", "C"];

struct Binding {
    raw: String,
    mods: Vec<String>,
    keyname: Option<String>,
}

struct BindingsInput(pub(crate) Vec<Binding>);

impl Parse for BindingsInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut bindings = as_bindings(comma_sep_strs(input)?);

        let templated_content;
        parenthesized!(templated_content in input);

        while !templated_content.is_empty() {
            let content;
            parenthesized!(content in templated_content);
            bindings.extend(expand_templates(
                comma_sep_strs(&content)?,
                comma_sep_strs(&content)?,
            ));
        }

        Ok(Self(bindings))
    }
}

fn comma_sep_strs(input: ParseStream<'_>) -> Result<Vec<String>> {
    let content;
    parenthesized!(content in input);
    Ok(Punctuated::<LitStr, Token![,]>::parse_terminated(&content)?
        .iter()
        .map(LitStr::value)
        .collect())
}

fn as_bindings(raw: Vec<String>) -> Vec<Binding> {
    raw.iter()
        .map(|s| {
            let mut parts: Vec<&str> = s.split('-').collect();
            let (keyname, mods) = if parts.len() <= 1 {
                (Some(s.clone()), vec![])
            } else {
                (
                    parts.pop().map(String::from),
                    parts.into_iter().map(String::from).collect(),
                )
            };

            Binding {
                raw: s.clone(),
                keyname,
                mods,
            }
        })
        .collect()
}

fn expand_templates(templates: Vec<String>, keynames: Vec<String>) -> Vec<Binding> {
    templates
        .iter()
        .flat_map(|t| {
            let mut parts: Vec<&str> = t.split('-').collect();
            if parts.pop() != Some("{}") {
                panic!(
                    "'{}' is an invalid template: expected '<Modifiers>-{{}}'",
                    t
                )
            };
            keynames
                .iter()
                .map(|k| Binding {
                    raw: format!("{}-{}", parts.join("-"), k),
                    mods: parts.iter().map(|m| m.to_string()).collect(),
                    keyname: Some(k.into()),
                })
                .collect::<Vec<Binding>>()
        })
        .collect()
}

fn has_valid_modifiers(binding: &Binding) -> bool {
    binding
        .mods
        .iter()
        .all(|s| VALID_MODIFIERS.contains(&s.as_ref()))
}

fn is_valid_keyname(binding: &Binding, names: &[String]) -> bool {
    if let Some(ref k) = binding.keyname {
        names.contains(&k)
    } else {
        false
    }
}

fn report_error(msg: impl AsRef<str>, b: &Binding) {
    panic!(
        "'{}' is an invalid key binding: {}\n\
        Key bindings should be of the form <modifiers>-<key name> or <key name> e.g:  M-j, M-S-slash, M-C-Up, XF86AudioMute",
        b.raw,
        msg.as_ref()
    )
}

pub(crate) fn validate_user_bindings_inner(input: TokenStream) -> TokenStream {
    let BindingsInput(mut bindings) = parse_macro_input!(input as BindingsInput);
    let names: Vec<String> = XKeySym::iter().map(|x| x.as_ref().to_string()).collect();
    let mut seen = HashSet::new();

    for b in bindings.iter_mut() {
        if seen.contains(&b.raw) {
            panic!("'{}' is bound as a keybinding more than once", b.raw);
        } else {
            seen.insert(&b.raw);
        }

        if b.keyname.is_none() {
            report_error("no key name specified", b)
        }

        if !is_valid_keyname(b, &names) {
            report_error(
                format!(
                    "'{}' is not a known key: run 'xmodmap -pke' to see valid key names",
                    b.keyname.take().unwrap()
                ),
                b,
            )
        }

        if !has_valid_modifiers(b) {
            report_error(
                format!(
                    "'{}' is an invalid modifer set: valid modifiers are {:?}",
                    b.mods.join("-"),
                    VALID_MODIFIERS
                ),
                b,
            );
        }
    }

    // If everything is fine then just consume the input
    TokenStream::new()
}
