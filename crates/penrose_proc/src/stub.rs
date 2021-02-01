//! Auto generation of stub/mock companion traits
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    fold::Fold, parse_macro_input, punctuated::Punctuated, token::Comma, Block, Error, Expr,
    ExprMethodCall, FnArg, Ident, ItemTrait, Lit, Meta, ReturnType, TraitItem, TraitItemMethod,
};

const STUB_METHOD_PREFIX: &str = "mock";
const DEFAULT_PREFIX: &str = "Stub";

// Data is the set of method names to replace
struct DefaultImplRewriter(Vec<String>);

impl Fold for DefaultImplRewriter {
    fn fold_expr_method_call(&mut self, m: ExprMethodCall) -> ExprMethodCall {
        if let Expr::Path(ref p) = *m.receiver {
            if let Some(ident) = p.path.get_ident() {
                if ident == "self" && self.0.contains(&m.method.to_string()) {
                    let mut stub = m.clone();
                    stub.method = format_ident!("{}_{}", STUB_METHOD_PREFIX, stub.method);
                    return stub;
                }
            }
        }
        m
    }
}

struct MethodDetails {
    ident: Ident,
    stub_ident: Ident,
    inputs: Punctuated<FnArg, Comma>,
    output: ReturnType,
    default: Option<Block>,
    stub: proc_macro2::TokenStream,
}

impl MethodDetails {
    fn rewrite_default(&mut self, trait_methods: &[String]) {
        self.default = self
            .default
            .take()
            .map(|d| DefaultImplRewriter(trait_methods.to_vec()).fold_block(d));
    }
}

fn strip_stub_attr(m: &mut TraitItemMethod) -> proc_macro2::TokenStream {
    let mut stub = None;

    for (ix, attr) in m.attrs.iter().enumerate() {
        let segs = &attr.path.segments;
        if segs.len() == 1 && segs[0].ident == "stub" {
            stub = Some((
                ix,
                match attr.parse_args::<Expr>() {
                    Ok(expr) => quote! { #expr },
                    Err(_) => {
                        Error::new_spanned(attr, "expected `stub(\"...\")`").to_compile_error()
                    }
                },
            ));
            break;
        }
    }

    if let Some((ix, tokens)) = stub {
        m.attrs.remove(ix);
        return tokens;
    }

    Error::new_spanned(
        m,
        "require `stub(\"...\")` attribute when there is no default implementation",
    )
    .to_compile_error()
}

fn extract_method_details(ast: &mut ItemTrait) -> Vec<MethodDetails> {
    ast.items
        .iter_mut()
        .map(|item| {
            if let TraitItem::Method(m) = item {
                MethodDetails {
                    ident: m.sig.ident.clone(),
                    stub_ident: format_ident!("{}_{}", STUB_METHOD_PREFIX, m.sig.ident),
                    inputs: m.sig.inputs.clone(),
                    output: m.sig.output.clone(),
                    default: m.default.clone(),
                    stub: strip_stub_attr(m),
                }
            } else {
                panic!("only supported for normal trait methods");
            }
        })
        .collect()
}

fn parse_args_meta(meta: Meta) -> Result<String, TokenStream> {
    if let Meta::NameValue(ref nv) = meta {
        if nv.path.is_ident("prefix") {
            if let Lit::Str(ref s) = nv.lit {
                return Ok(s.value());
            }
        }
    }

    Err(TokenStream::from(
        Error::new_spanned(
            meta,
            "Expected #[stubbed_companion_trait] or #[stubbed_companion_trait(prefix = \"Foo\")]",
        )
        .to_compile_error(),
    ))
}

fn generate_stub_methods(method_details: &[MethodDetails]) -> Vec<proc_macro2::TokenStream> {
    method_details
        .iter()
        .map(|m| {
            let MethodDetails {
                stub_ident,
                inputs,
                output,
                stub,
                default,
                ..
            } = m;

            if let Some(body) = default {
                quote! {
                    fn #stub_ident(#inputs) #output {
                        #body
                    }
                }
            } else {
                quote! {
                    #[allow(unused_variables)]
                    fn #stub_ident(#inputs) #output {
                        #stub
                    }
                }
            }
        })
        .collect()
}

fn generate_trait_methods(method_details: &[MethodDetails]) -> Vec<proc_macro2::TokenStream> {
    method_details
        .iter()
        .map(|m| {
            let MethodDetails {
                ident,
                stub_ident,
                inputs,
                output,
                ..
            } = m;

            let params = inputs.iter().flat_map(|i| {
                if let FnArg::Typed(t) = i {
                    let param = t.pat.clone();
                    Some(quote! { #param })
                } else {
                    None
                }
            });

            quote! {
                fn #ident(#inputs) #output {
                   self.#stub_ident(#(#params),*)
                }
            }
        })
        .collect()
}

pub(crate) fn stubbed_companion_trait_inner(args: TokenStream, input: TokenStream) -> TokenStream {
    let prefix = if args.is_empty() {
        DEFAULT_PREFIX.to_string()
    } else {
        match parse_args_meta(parse_macro_input!(args as Meta)) {
            Ok(s) => s,
            Err(e) => return e,
        }
    };

    // Extract top level trait details
    let mut ast = parse_macro_input!(input as ItemTrait);
    let trait_name = ast.ident.clone();
    let stub_trait_name = format_ident!("{}{}", prefix, trait_name);
    let visibility = ast.vis.clone();
    let colon = ast.colon_token.clone();
    let bounds = ast.supertraits.clone();

    // Extract method details and then rewrite default impls to use our new stubbed methods
    // to avoid crazy trait bound errors
    let mut method_details = extract_method_details(&mut ast);
    let method_names: Vec<String> = method_details.iter().map(|m| m.ident.to_string()).collect();
    method_details
        .iter_mut()
        .for_each(|m| m.rewrite_default(&method_names));

    // Generate all of the output token streams that we need
    let stub_methods = generate_stub_methods(&method_details);
    let trait_methods = generate_trait_methods(&method_details);

    TokenStream::from(quote! {
        #ast

        #visibility trait #stub_trait_name #colon #bounds {
            #(#stub_methods)*
        }

        impl<T> #trait_name for T where T: #stub_trait_name {
            #(#trait_methods)*
        }
    })
}
