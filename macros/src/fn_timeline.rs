use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token, Error, FieldValue, Lit, LitByte, LitFloat, LitInt, Member, Path, Result, Token,
};

pub fn expand_timeline(name: &Path, config: &TimelineConfig) -> Result<TokenStream2> {
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
    let easing_setter = config.easing.as_ref().map(|easing| {
        quote! { .default_easing(#easing) }
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
            #easing_setter
            #repeat_setter
            #reverse_setter
            #(#keyframe_appenders)*
            .build()
    })
}

pub fn expand_timeline_or_merge(
    name: &Path,
    config: &TimelineOrMergeConfig,
) -> Result<TokenStream2> {
    if config.timelines.len() == 1 {
        expand_timeline(name, &config.timelines[0])
    } else {
        let timeline_creators = config
            .timelines
            .iter()
            .map(|cfg| expand_timeline(name, cfg))
            .collect::<Result<Vec<_>>>()?;
        Ok(quote! {
            ::mina::MergedTimeline::of([#(#timeline_creators),*])
        })
    }
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

fn seconds_multiplier(num_lit: &NumericLit) -> Result<f32> {
    match num_lit.suffix() {
        "s" => Ok(1.0),
        "ms" => Ok(0.001),
        _ => Err(Error::new(num_lit.span(), "blah")),
    }
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
pub struct TimelineOrMergeConfig {
    pub timelines: Vec<TimelineConfig>,
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
pub struct TimelineConfig {
    pub _span: Span,
    pub duration: Option<TimelineDurationArgument>,
    pub delay: Option<TimelineDelayArgument>,
    pub easing: Option<Path>,
    pub repeat: Option<KeyframeRepeatArgument>,
    pub reverse: Option<kw::reverse>,
    pub keyframes: Vec<KeyframeConfig>,
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
            } else if input.fork().parse::<Path>().is_ok() {
                // Can't peek on a Path (probably too complex/expensive?), so we have to attempt an
                // actual parse and fail gracefully if it's not a path. This branch goes last, i.e.
                // only runs if nothing else can match and we're about to fail anyway.
                config.easing = Some(input.parse::<Path>()?);
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
pub enum NumericLit {
    Byte(LitByte),
    Int(LitInt),
    Float(LitFloat),
}

impl NumericLit {
    pub fn as_f32(&self) -> Result<f32> {
        let value = match self {
            NumericLit::Byte(lit_byte) => lit_byte.value() as f32,
            NumericLit::Int(lit_int) => lit_int.base10_parse()?,
            NumericLit::Float(lit_float) => lit_float.base10_parse()?,
        };
        Ok(value)
    }

    pub fn span(&self) -> Span {
        match self {
            NumericLit::Byte(lit_byte) => lit_byte.span(),
            NumericLit::Int(lit_int) => lit_int.span(),
            NumericLit::Float(lit_float) => lit_float.span(),
        }
    }

    pub fn suffix(&self) -> &str {
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
pub struct TimelineDurationArgument {
    pub _prefix: Option<Token![for]>,
    pub value: NumericLit,
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
pub struct TimelineDelayArgument {
    pub _prefix: kw::after,
    pub value: NumericLit,
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
pub struct KeyframeConfig {
    pub position: KeyframePositionArgument,
    pub values: KeyframeValues,
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
pub enum KeyframePositionArgument {
    From(kw::from),
    To(kw::to),
    Percent(NumericLit, Token![%]),
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
pub enum KeyframeRepeatArgument {
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
pub enum KeyframeValues {
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
            let values = Punctuated::parse_terminated(&content)?;
            Ok(KeyframeValues::Explicit(values, brace_token))
        }
    }
}

pub mod kw {
    use syn::custom_keyword;

    custom_keyword!(from); // Keyframe at 0%
    custom_keyword!(to); // Keyframe at 100%
    custom_keyword!(after); // Timeline delay
    custom_keyword!(reverse); // Timeline auto-reverses
    custom_keyword!(infinite); // Timeline repeats infinitely
}
