//! Macro to do a lot of the legwork of implementing `stef::State`
//! and interfacing with that implementation.
//!
//! Normally, to implement `State`, you must:
//! - define an Action type
//! - define an Effect type
//! - implement the `State::transition` function, taking an Action and returning an Effect
//!
//! This is uncomfortable to work with though: rather than calling functions on your state,
//! you always have to do an explicit `transition`, using an enum instead of normal function args.
//!
//! This macro lets you define your Actions as individual functions returning Effects, rather than
//! needing to define a single transition function to handle each Action. The macro generates an
//! Action type for you based on the functions you define, and the functions get rewritten such that:
//!
//! - Each input signature corresponds to an Action variant
//! - The function maps its inputs into the correct Action variant and passes that to `State::transition`
//! - The `State::transition` function is written for you automatically
//! - Optionally, each function can specify a "matches" pattern, for situations where a particular
//!     action can only ever produce a subset of possible effects. This allows the functions to return
//!     something other than the Effect type, which makes calling the function more ergonomic, so that
//!     you don't have to re-map the Effect at the call site. (Under the hood, every Action still
//!     produces the same Effect type.)
//!
//! ## Implementation details
//!
//! To help with following along with what this macro is doing, the rewriting is done roughly as follows:
//!
//! - The `type Action` and `type Effect` statements are parsed to learn those types
//! - The function definitions are collected
//! - The Action enum is built up, with each variant defined according to the function inputs
//! - The original `impl<_> State` block is gutted and replaced with a `transition` fn, completing the State implementation
//! - A new `impl` block is created containing the original function bodies, but with the function names prefixed by `_stef_impl_`
//!     and private visibility (these should never be leaked or otherwise called directly, it would defeat the entire purpose!)
//! - Another new `impl` block is created with the original function names, but with bodies that simply call the `transition`
//!     function with the `Action` corresponding to this function. If a `matches` directive was provided, the pattern is
//!     applies to the output to map the return type

use heck::ToPascalCase;
use proc_macro2::{Span, TokenStream};
use proc_macro_error::abort;
use quote::{quote, ToTokens};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Ident, Pat, Token, Type};

#[proc_macro_attribute]
#[proc_macro_error::proc_macro_error]
pub fn state(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    if let Some(proc_macro::TokenTree::Ident(i)) = item.clone().into_iter().next() {
        if &i.to_string() == "impl" {
            return state_impl(attr, item);
        }
    }
    item
}

struct Options {
    _parameterized: Option<syn::Path>,
}

impl syn::parse::Parse for Options {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut _parameterized = None;

        while !input.is_empty() {
            let key: syn::Path = input.parse()?;
            if key.is_ident("parameterized") {
                _parameterized = Some(input.parse()?);
            }
            let _: syn::Result<syn::Token![,]> = input.parse();
        }

        Ok(Self { _parameterized })
    }
}

struct MatchPat {
    var: Ident,
    pat: Pat,
}

impl syn::parse::Parse for MatchPat {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pat = Pat::parse_single(input)?;
        if let (Ok(_), Ok(_)) = (input.parse::<Token!(=)>(), input.parse::<Token!(>)>()) {
        } else {
            return Err(syn::Error::new(pat.span(), "Expected => in `matches()`"));
        }
        let var = input.parse()?;
        Ok(Self { var, pat })
    }
}

fn state_impl(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let Options { _parameterized: _ } = parse_macro_input!(attr as Options);

    let mut action_name = None;
    let mut effect_name = None;

    let item = parse_macro_input!(input as syn::ItemImpl);

    // let struct_ty = item.self_ty.clone();
    let struct_path = match &*item.self_ty {
        Type::Path(path) => path,
        _ => abort!(item.self_ty.span(), "This impl is too fancy"),
    };

    struct F {
        f: syn::ImplItemFn,
        match_pat: Option<MatchPat>,
    }

    impl F {
        fn _name(&self) -> Ident {
            syn::Ident::new(&self.f.sig.ident.to_string(), self.f.span())
        }
        fn impl_name(&self) -> Ident {
            syn::Ident::new(&format!("_stef_impl_{}", self.f.sig.ident), self.f.span())
        }
        fn variant_name(&self) -> Ident {
            syn::Ident::new(
                &self.f.sig.ident.to_string().to_pascal_case(),
                self.f.span(),
            )
        }
        fn inputs(&self) -> Vec<(Box<Pat>, Box<Type>)> {
            let mut f_inputs = self.f.sig.inputs.iter();
            match f_inputs.next().expect("problem 1!") {
                syn::FnArg::Receiver(r) if r.mutability.is_some() => (),
                o => {
                    abort!(
                        o.span(),
                        "#[stef::state] must take &mut self as first argument",
                    )
                }
            }

            f_inputs
                .map(|i| match i {
                    syn::FnArg::Typed(arg) => (arg.pat.clone(), arg.ty.clone()),
                    _ => unreachable!(),
                })
                .collect()
        }
    }

    let mut fns = vec![];

    for item in item.items {
        match item {
            syn::ImplItem::Type(ty) => {
                if ty.ident == "Action" {
                    action_name = Some(ty.ty.clone());
                } else if ty.ident == "Effect" {
                    effect_name = Some(ty.ty.clone());
                }
            }
            syn::ImplItem::Fn(f) => {
                let span = f.span();

                let mut match_pat = None;
                for attr in f.attrs.iter() {
                    if attr.path().segments.last().map(|s| s.ident.to_string())
                        == Some("state".to_string())
                    {
                        attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident("matches") {
                                let content;
                                syn::parenthesized!(content in meta.input);
                                let mp: MatchPat =
                                    content.parse().map_err(|e| syn::Error::new(span, e))?;
                                match_pat = Some(mp);
                                return Ok(());
                            }
                            Ok(())
                        })
                        .unwrap_or_else(|err| {
                            abort!("blah {}", err);
                        })
                    }
                }

                fns.push(F { f, match_pat });
            }
            _ => {}
        }
    }

    let action_name =
        action_name.unwrap_or_else(|| abort!(Span::call_site(), "`type Action` must be set"));
    let effect_name =
        effect_name.unwrap_or_else(|| abort!(Span::call_site(), "`type Effect` must be set"));

    let define_action_enum_variants = delim::<_, Token!(,)>(fns.iter().map(|f| {
        let args = delim::<_, Token!(,)>(f.inputs().into_iter().map(|(_, ty)| ty));
        let variant_name = f.variant_name();
        if args.is_empty() {
            f.variant_name().to_token_stream()
        } else {
            quote! ( #variant_name(#args) ).to_token_stream()
        }
    }));

    let mut define_action_enum: syn::ItemEnum = syn::parse(
        quote! {
            pub enum #action_name {
                #define_action_enum_variants
            }
        }
        .into(),
    )
    .expect("problem 2!");
    define_action_enum.generics = item.generics.clone();

    let define_hidden_fns_inner = ss_flatten(fns.iter().map(|f| {
        let mut original_func = f.f.clone();
        let (rarr, output_type) = match &mut original_func.sig.output {
            syn::ReturnType::Default => abort!(f.f.span(), "functions must return a type"),
            syn::ReturnType::Type(rarr, t) => {
                let actual = t.clone();

                // *t = Box::new(effect_name.clone());

                (rarr, actual)
            }
        };
        original_func.sig.ident = syn::Ident::new(
            &format!("_stef_impl_{}", original_func.sig.ident),
            f.f.span(),
        );
        original_func.sig.output = syn::ReturnType::Type(*rarr, output_type);
        // let impl_name = f.impl_name();
        // let block = f.f.block;
        // let args = delim::<_, Token!(,)>(f.inputs().iter().map(|(pat, ty)| quote! { #pat: #ty }));
        original_func.to_token_stream()
        // quote! {
        //    fn #impl_name(&mut self, #args) -> #output_type {
        //        #block
        //    }
        // }
    }));

    let define_public_fns_inner = ss_flatten(fns.iter().map(|f| {
        let mut original_func = f.f.clone();
        let variant_name = f.variant_name();

        let pats = delim::<_, Token!(,)>(f.inputs().into_iter().map(|(pat, _)| pat));
        let arg = match pats.is_empty() {
            true => quote! { <Self as stef::State>::Action::#variant_name },
            false => quote! { <Self as stef::State>::Action::#variant_name(#pats) },
        };

        let new_block = if let Some(MatchPat { var, pat }) = f.match_pat.as_ref() {
            quote! {{
                use stef::State;
                let eff = self.transition(#arg);
                match eff {
                    #pat => #var,
                    _ => unreachable!("stef::state has a bug in its effect unwrapping logic")
                }
            }}
        } else {
            quote! {{
                use stef::State;
                self.transition(#arg)
            }}
        };

        let ts = proc_macro::TokenStream::from(new_block.into_token_stream());
        original_func.block = syn::parse(ts).expect("problem 3!");
        original_func.to_token_stream()
    }));

    let mut define_hidden_fns: syn::ItemImpl = syn::parse(
        quote! {
            impl #struct_path {
                #define_hidden_fns_inner
            }
        }
        .into(),
    )
    .expect("problem 4!");
    define_hidden_fns.generics = item.generics.clone();

    let mut define_public_fns: syn::ItemImpl = syn::parse(
        quote! {
            impl #struct_path {
                #define_public_fns_inner
            }
        }
        .into(),
    )
    .expect("problem 5!");
    define_public_fns.generics = item.generics.clone();

    // let action_name_generic = match action_name.clone() {
    //     Type::Path(mut path) => {
    //         path.path.segments.last_mut().expect("problem 6!").arguments = struct_path
    //             .path
    //             .segments
    //             .last()
    //             .expect("problem 7!")
    //             .arguments
    //             .clone();
    //         path
    //     }
    //     _ => todo!(),
    // };
    let _action_name_nogeneric = match action_name.clone() {
        Type::Path(mut path) => {
            path.path.segments.last_mut().expect("problem 6!").arguments = Default::default();
            path
        }
        _ => todo!(),
    };

    let define_transitions = delim::<_, Token!(,)>(fns.iter().map(|f| {
        let args = delim::<_, Token!(,)>(f.inputs().into_iter().map(|(pat, _)| pat));
        let variant_name = f.variant_name();
        let impl_name = f.impl_name();
        if args.is_empty() {
            quote! {
                Self::Action::#variant_name => self.#impl_name().into()
            }
        } else {
            quote! {
                Self::Action::#variant_name(#args) => self.#impl_name(#args).into()
            }
        }
    }));

    let mut define_state_impl: syn::ItemImpl = syn::parse(
        quote! {
            impl stef::State for #struct_path {
                type Action = #action_name;
                type Effect = #effect_name;

                fn transition(&mut self, action: Self::Action) -> Self::Effect {
                    match action {
                        #define_transitions
                    }
                }
            }
        }
        .into(),
    )
    .expect("problem 8!");

    define_state_impl.generics = item.generics;

    let expanded = quote! {
        #define_action_enum
        #define_public_fns
        #define_hidden_fns
        #define_state_impl
    };

    proc_macro::TokenStream::from(expanded)
}

fn delim<T: ToTokens, P: ToTokens + Default>(ss: impl Iterator<Item = T>) -> Punctuated<T, P> {
    let mut items = Punctuated::<T, P>::new();
    items.extend(ss);
    items
}

fn ss_flatten(ss: impl Iterator<Item = TokenStream>) -> TokenStream {
    ss.fold(TokenStream::new(), |mut ss, s| {
        ss.extend(s);
        ss
    })
}