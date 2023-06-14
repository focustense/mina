use crate::fn_timeline::{expand_timeline_or_merge, TimelineOrMergeConfig};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token, Error, Expr, FieldValue, Member, Path, Result, Token, Type,
};

pub fn animator_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as AnimatorInput);
    expand_animator(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_animator(input: AnimatorInput) -> Result<TokenStream2> {
    let AnimatorInput {
        target_type,
        defaults,
        states,
        ..
    } = input;
    let Type::Path(ref type_path) = target_type else {
        return Err(Error::new(target_type.span(), "Animator macro only supports use-types and type paths."));
    };
    let name = &type_path.path;
    let default_state_assignment = defaults
        .as_ref()
        .map(|def| &def.state)
        .map(|def_state| quote! { .from_state(#def_state) });
    let default_values_assignment = match defaults.as_ref().map(|def| &def.values) {
        Some(AnimatorDefaultValues::Expr(expr)) => quote! { #expr },
        Some(AnimatorDefaultValues::Inline(field_values, _)) => {
            inline_defaults(name, field_values)?
        }
        _ => quote! { #target_type::default() },
    };
    let mut state_assignments = Vec::new();
    for state_mapping in &states {
        let timeline = expand_timeline_or_merge(name, &state_mapping.behavior)?;
        for state in &state_mapping.states {
            state_assignments.push(quote! { .on(#state, #timeline) })
        }
    }
    let anim = quote! {
        {
            let default_values = #default_values_assignment;
            ::mina::StateAnimatorBuilder::new()
                #default_state_assignment
                .from_values(default_values.clone())
                #(#state_assignments)*
                .build()
        }
    };
    Ok(anim)
}

fn inline_defaults(
    name: &Path,
    field_values: &Punctuated<FieldValue, Token![,]>,
) -> Result<TokenStream2> {
    let assignments = field_values
        .iter()
        .map(|fv| {
            let Member::Named(field_name) = &fv.member else {
                return Err(Error::new(fv.span(), "Animator macro only supports named fields."));
            };
            let expr = &fv.expr;
            Ok(quote! { default_values.#field_name = #expr })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(quote! {
        {
            let mut default_values = #name::default();
            #(#assignments);*;
            default_values
        }
    })
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
struct AnimatorInput {
    target_type: Type,
    defaults: Option<AnimatorDefaults>,
    _brace_token: token::Brace,
    states: Punctuated<AnimatorStateMapping, Token![,]>,
}

impl Parse for AnimatorInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            target_type: input.parse()?,
            _brace_token: braced!(content in input),
            defaults: if content.peek(Token![default]) {
                Some(content.parse()?)
            } else {
                None
            },
            states: content.parse_terminated(AnimatorStateMapping::parse, Token![,])?,
        })
    }
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
struct AnimatorDefaults {
    _default_token: Token![default],
    _paren_token: token::Paren,
    _terminator: Token![,],
    state: Path,
    values: AnimatorDefaultValues,
}

impl Parse for AnimatorDefaults {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _default_token = input.parse::<Token![default]>()?;
        let _paren_token = parenthesized!(content in input);
        let state = content.parse::<Path>()?;
        let values = if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
            content.parse::<AnimatorDefaultValues>()?
        } else {
            AnimatorDefaultValues::None
        };
        Ok(Self {
            _default_token,
            _paren_token,
            state,
            values,
            _terminator: input.parse()?,
        })
    }
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
enum AnimatorDefaultValues {
    None,
    Expr(Expr),
    Inline(Punctuated<FieldValue, Token![,]>, token::Brace),
}

impl Parse for AnimatorDefaultValues {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.cursor().eof() {
            Ok(Self::None)
        } else if input.peek(token::Brace) {
            let content;
            let brace_token = braced!(content in input);
            let values = Punctuated::parse_terminated(&content)?;
            Ok(Self::Inline(values, brace_token))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
struct AnimatorStateMapping {
    states: Punctuated<Path, Token![|]>,
    behavior: TimelineOrMergeConfig,
}

impl Parse for AnimatorStateMapping {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let states = Punctuated::<Path, Token![|]>::parse_separated_nonempty(input)?;
        input.parse::<Token![=>]>()?;
        Ok(Self {
            states,
            behavior: input.parse()?,
        })
    }
}
