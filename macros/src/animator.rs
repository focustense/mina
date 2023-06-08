use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token, Error, Expr, FieldValue, Ident, Lit, LitByte, LitFloat, LitInt, Member, Path, Result,
    Token, Type,
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
            inline_defaults(&name, &field_values)?
        }
        _ => quote! { #target_type::default() },
    };
    let mut state_assignments = Vec::new();
    for state_mapping in &states {
        let timeline = builder_create_timeline_or_merge(&name, &state_mapping.behavior)?;
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
    Ok(anim.into())
}

fn builder_append_keyframe(name: &Path, config: &KeyframeConfig) -> Result<TokenStream2> {
    let normalized_time = match &config.position {
        KeyframePositionArgument::From(_) => 0.0,
        KeyframePositionArgument::To(_) => 1.0,
        KeyframePositionArgument::Percent(lit, _) => lit.as_f32()? * 0.01,
    };
    match &config.values {
        KeyframeValues::Default(_) => Ok(quote! {
            .keyframe(#name::keyframe(#normalized_time)
                .values_from(#normalized_time, &default_values))
        }),
        KeyframeValues::Explicit(field_values, _) => {
            let setters = field_values
                .iter()
                .map(|fv| {
                    let Member::Named(field_name) = &fv.member else {
                        return Err(Error::new(fv.span(), "Animator macro only supports named fields."));
                    };
                    let expr = &fv.expr;
                    Ok(quote! { .#field_name(#expr) })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(quote! {
                .keyframe(#name::keyframe(#normalized_time)#(#setters)*)
            })
        }
    }
}

fn builder_create_timeline(name: &Path, config: &TimelineConfig) -> Result<TokenStream2> {
    let duration = match &config.duration {
        Some(duration) => Some(duration.value.as_f32()? * seconds_multiplier(&duration.value)?),
        None => None,
    };
    let duration_setter = duration.map(|duration_seconds| {
        quote! { .duration_seconds(#duration_seconds) }
    });
    let delay = match &config.delay {
        Some(delay) => Some(delay.value.as_f32()? * seconds_multiplier(&delay.value)?),
        None => None,
    };
    let delay_setter = delay.map(|delay_seconds| {
        quote! { .delay_seconds(#delay_seconds) }
    });
    let repeat_setter = match &config.repeat {
        Some(KeyframeRepeatArgument::Fixed(lit_int)) => {
            let times: u32 = lit_int.base10_parse()?;
            Some(quote! { .repeat(::mina::Repeat::Times(#times)) })
        }
        Some(KeyframeRepeatArgument::Infinite(_)) => {
            Some(quote! { .repeat(::mina::Repeat::Infinite) })
        }
        _ => None,
    };
    let reverse_setter = config.reverse.map(|_| quote! { .reverse(true) });
    let keyframe_appenders = config
        .keyframes
        .iter()
        .map(|kf| builder_append_keyframe(name, kf))
        .collect::<Result<Vec<_>>>()?;
    Ok(quote! {
        #name::timeline()
            #duration_setter
            #delay_setter
            #repeat_setter
            #reverse_setter
            #(#keyframe_appenders)*
    })
}

fn builder_create_timeline_or_merge(
    name: &Path,
    config: &TimelineOrMergeConfig,
) -> Result<TokenStream2> {
    if config.timelines.len() == 1 {
        builder_create_timeline(name, &config.timelines[0])
    } else {
        let timeline_creators = config
            .timelines
            .iter()
            .map(|cfg| builder_create_timeline(name, cfg))
            .collect::<Result<Vec<_>>>()?;
        Ok(quote! {
            ::mina::MergedTimeline::of([#(#timeline_creators),*])
        })
    }
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

fn seconds_multiplier(num_lit: &NumericLit) -> Result<f32> {
    match num_lit.suffix() {
        "s" => Ok(1.0),
        "ms" => Ok(1000.0),
        _ => Err(Error::new(num_lit.span(), "blah")),
    }
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

mod kw {
    use syn::custom_keyword;

    custom_keyword!(from); // Keyframe at 0%
    custom_keyword!(to); // Keyframe at 100%
    custom_keyword!(after); // Timeline delay
    custom_keyword!(reverse); // Timeline auto-reverses
    custom_keyword!(infinite); // Timeline repeats infinitely
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
            let values = Punctuated::parse_separated_nonempty(&content)?;
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

#[cfg_attr(feature = "parse-debug", derive(Debug))]
struct TimelineOrMergeConfig {
    timelines: Vec<TimelineConfig>,
}

impl Parse for TimelineOrMergeConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(token::Bracket) {
            let content;
            let _ = bracketed!(content in input);
            let timelines =
                Punctuated::<TimelineConfig, Token![,]>::parse_separated_nonempty(&content)?;
            Ok(Self {
                timelines: timelines.into_iter().collect(),
            })
        } else {
            Ok(Self {
                timelines: vec![input.parse()?],
            })
        }
    }
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
struct TimelineConfig {
    _span: Span,
    duration: Option<TimelineDurationArgument>,
    delay: Option<TimelineDelayArgument>,
    easing: Option<Ident>,
    repeat: Option<KeyframeRepeatArgument>,
    reverse: Option<kw::reverse>,
    keyframes: Vec<KeyframeConfig>,
}

impl TimelineConfig {
    fn new(span: Span) -> Self {
        Self {
            _span: span,
            duration: None,
            delay: None,
            easing: None,
            repeat: None,
            reverse: None,
            keyframes: Vec::new(),
        }
    }
}

impl Parse for TimelineConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut config = TimelineConfig::new(input.span());
        loop {
            if input.peek(Token![,]) || input.cursor().eof() {
                break;
            } else if input.peek(Token![for]) {
                config.duration = Some(input.parse()?);
            } else if input.peek(kw::after) {
                config.delay = Some(input.parse()?);
            } else if input.peek(kw::reverse) {
                config.reverse = Some(input.parse()?);
            } else if input.peek(kw::infinite) {
                config.repeat = Some(input.parse()?);
            } else if input.peek(kw::from) || input.peek(kw::to) {
                config.keyframes.push(input.parse()?);
            } else if input.peek(Ident) {
                config.easing = Some(input.parse()?);
            } else if input.peek(Lit) {
                let lookahead_input = input.fork();
                let lit = lookahead_input.parse::<Lit>()?;
                match lit.suffix() {
                    "s" | "ms" => config.duration = Some(input.parse()?),
                    "x" => config.repeat = Some(input.parse()?),
                    "" if lookahead_input.peek(Token![%]) => config.keyframes.push(input.parse()?),
                    _ => {
                        return Err(Error::new(
                            input.span(),
                            concat!(
                                "Timeline argument has no prefix and unrecognized suffix. ",
                                "Supported suffixes are 's' or 'ms' for duration, 'x' for repeat ",
                                "count or '%' for keyframes."
                            ),
                        ))
                    }
                }
            } else {
                return Err(Error::new(
                    input.span(),
                    concat!(
                        "Token type is not supported in timeline syntax. Expected one of: ",
                        "[for] {duration}, after {delay}, {Easing}, reverse, {repeat}x, infinite, ",
                        "from {keyframe}, to {keyframe}, or {pos}% {keyframe}."
                    ),
                ));
            }
        }
        Ok(config)
    }
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
enum NumericLit {
    Byte(LitByte),
    Int(LitInt),
    Float(LitFloat),
}

impl NumericLit {
    fn as_f32(&self) -> Result<f32> {
        let value = match self {
            NumericLit::Byte(lit_byte) => lit_byte.value() as f32,
            NumericLit::Int(lit_int) => lit_int.base10_parse()?,
            NumericLit::Float(lit_float) => lit_float.base10_parse()?,
        };
        Ok(value)
    }

    fn span(&self) -> Span {
        match self {
            NumericLit::Byte(lit_byte) => lit_byte.span(),
            NumericLit::Int(lit_int) => lit_int.span(),
            NumericLit::Float(lit_float) => lit_float.span(),
        }
    }

    fn suffix(&self) -> &str {
        match self {
            NumericLit::Byte(lit_byte) => lit_byte.suffix(),
            NumericLit::Int(lit_int) => lit_int.suffix(),
            NumericLit::Float(lit_float) => lit_float.suffix(),
        }
    }
}

impl Parse for NumericLit {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lit = input.parse::<Lit>()?;
        match lit {
            Lit::Byte(lit_byte) => Ok(NumericLit::Byte(lit_byte)),
            Lit::Int(lit_int) => Ok(NumericLit::Int(lit_int)),
            Lit::Float(lit_float) => Ok(NumericLit::Float(lit_float)),
            _ => Err(Error::new(
                lit.span(),
                "Literal in this position must be a numeric type.",
            )),
        }
    }
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
struct TimelineDurationArgument {
    _prefix: Option<Token![for]>,
    value: NumericLit,
}

impl Parse for TimelineDurationArgument {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let prefix = if input.peek(Token![for]) {
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self {
            _prefix: prefix,
            value: input.parse()?,
        })
    }
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
struct TimelineDelayArgument {
    _prefix: kw::after,
    value: NumericLit,
}

impl Parse for TimelineDelayArgument {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _prefix: input.parse()?,
            value: input.parse()?,
        })
    }
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
struct KeyframeConfig {
    position: KeyframePositionArgument,
    values: KeyframeValues,
}

impl Parse for KeyframeConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let position: KeyframePositionArgument;
        if input.peek(kw::from) {
            position = KeyframePositionArgument::From(input.parse()?);
        } else if input.peek(kw::to) {
            position = KeyframePositionArgument::To(input.parse()?);
        } else if input.peek(Lit) {
            let num_lit = input.parse::<NumericLit>()?;
            let percent_token = input.parse::<Token![%]>()?;
            position = KeyframePositionArgument::Percent(num_lit, percent_token);
        } else {
            return Err(Error::new(
                input.span(),
                concat!(
                    "Invalid keyframe position; expected the keyword 'from', 'to' or a number ",
                    "ending in %"
                ),
            ));
        }
        Ok(Self {
            position,
            values: input.parse()?,
        })
    }
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
enum KeyframePositionArgument {
    From(kw::from),
    To(kw::to),
    Percent(NumericLit, Token![%]),
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
enum KeyframeRepeatArgument {
    Fixed(LitInt),
    Infinite(kw::infinite),
}

impl Parse for KeyframeRepeatArgument {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(kw::infinite) {
            Ok(Self::Infinite(input.parse()?))
        } else {
            let lit = input.parse::<Lit>()?;
            if let Lit::Int(lit_int) = lit {
                Ok(Self::Fixed(lit_int))
            } else {
                Err(Error::new(
                    lit.span(),
                    "Repeat argument must be an integer literal",
                ))
            }
        }
    }
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
enum KeyframeValues {
    Default(Token![default]),
    Explicit(Punctuated<FieldValue, Token![,]>, token::Brace),
}

impl Parse for KeyframeValues {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![default]) {
            Ok(KeyframeValues::Default(input.parse()?))
        } else {
            let content;
            let brace_token = braced!(content in input);
            let values = Punctuated::parse_separated_nonempty(&content)?;
            Ok(KeyframeValues::Explicit(values, brace_token))
        }
    }
}
