use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{
    parse2, parse_macro_input, parse_str, spanned::Spanned, Data, DeriveInput, Error, Field,
    Fields, Lit, Meta, Path, Result, Token, Visibility,
};

pub fn animate_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_animate(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_animate(input: DeriveInput) -> Result<TokenStream2> {
    let DeriveInput {
        ident: name,
        data,
        generics: _generics, // Not supported yet
        attrs,
        vis,
    } = input;
    let Data::Struct(struct_data) = data else {
        return Err(Error::new(
            Span::call_site(),
            "derive(Animate) requires a struct type.",
        ));
    };
    let Fields::Named(fields) = struct_data.fields else {
        return Err(Error::new(
            struct_data.fields.span(),
            "derive(Animate) requires a struct with named fields.",
        ));
    };

    let mut remote_path = Path::from(name.clone());
    for attr in &attrs {
        let Meta::List(ref list) = attr.meta else {
            continue;
        };
        if !is_simple_path(&list.path, "animate") {
            continue;
        }
        let parsed_attr = parse2::<AnimateAttributeInput>(list.tokens.clone())?;
        let attr_name = parsed_attr.name.to_string();
        match attr_name.as_str() {
            "remote" => {
                let Lit::Str(value) = parsed_attr.value else {
                    return Err(Error::new(
                        parsed_attr.span,
                        "Expected value of 'remote' attribute to be a string.",
                    ));
                };
                remote_path = parse_str::<Path>(&value.value())?;
            }
            _ => {
                return Err(Error::new(
                    list.span(),
                    format!("Unrecognized animation attribute: {attr_name}"),
                ))
            }
        };
    }
    let remote_name = &remote_path.segments.last().unwrap().ident;

    let anim_fields = fields
        .named
        .iter()
        .filter(|f| is_animatable(f))
        .collect::<Vec<_>>();
    let anim_fields = if anim_fields.is_empty() {
        fields.named.iter().collect()
    } else {
        anim_fields
    };

    let builder_shortcuts = builder_shortcuts(&name, remote_name, &anim_fields);
    let timeline_struct = timeline_struct(remote_name, &vis, &anim_fields)?;
    let timeline_builder_impl = timeline_builder_impl(remote_name, &anim_fields);
    let keyframe_struct = keyframe_struct(remote_name, &vis, &anim_fields);
    let keyframe_builder = keyframe_builder(&remote_path, &vis, &anim_fields);
    let animate = quote! {
        #builder_shortcuts
        #timeline_struct
        #timeline_builder_impl
        #keyframe_struct
        #keyframe_builder
    };

    Ok(animate)
}

fn builder_shortcuts(
    target_name: &Ident,
    remote_name: &Ident,
    target_fields: &[&Field],
) -> TokenStream2 {
    let builder_name = format_ident!("{remote_name}KeyframeBuilder");
    let data_name = format_ident!("{remote_name}KeyframeData");
    // When using remote, the decorated struct's fields are never accessed directly.
    // This pattern is used to prevent dead code warnings and is adapted from Serde's version:
    // https://github.com/serde-rs/serde/blob/9cdf33202977df68289a42b1ba30885b6b2abe44/serde_derive/src/pretend.rs
    let fake_access = if target_name != remote_name {
        let field_names = target_fields.iter().map(|f| &f.ident);
        let placeholders = (0usize..).map(|i| format_ident!("__v{}", i));
        quote! {
            match std::option::Option::None::<&#target_name> {
                std::option::Option::Some(#target_name { #(#field_names: #placeholders),*,.. }) => {},
                _ => {}
            }
        }
    } else {
        quote!()
    };
    quote! {
        impl #target_name {
            pub fn keyframe(normalized_time: f32) -> #builder_name {
                #builder_name::new(normalized_time)
            }

            pub fn timeline() -> ::mina::TimelineConfiguration<#data_name> {
                #fake_access
                ::mina::TimelineConfiguration::default()
            }
        }
    }
}

fn is_animatable(field: &Field) -> bool {
    field.attrs.iter().any(|attr| {
        let Meta::Path(ref path) = attr.meta else {
            return false;
        };
        is_simple_path(path, "animate")
    })
}

fn is_simple_path<'a>(path: &Path, name: impl Into<&'a str>) -> bool {
    path.segments.len() == 1
        && path.segments[0].arguments.is_none()
        && path.segments[0].ident == name.into()
}

fn keyframe_builder(
    remote_path: &Path,
    target_visibility: &Visibility,
    target_fields: &[&Field],
) -> TokenStream2 {
    let remote_name = &remote_path.segments.last().unwrap().ident;
    let builder_name = format_ident!("{remote_name}KeyframeBuilder");
    let data_name = format_ident!("{remote_name}KeyframeData");
    let setters = target_fields.iter().map(|f| {
        let Field {
            ident: field_name,
            ty,
            ..
        } = f;
        quote! {
            pub fn #field_name(mut self, #field_name: #ty) -> Self {
                self.data.#field_name = std::option::Option::Some(#field_name);
                self
            }
        }
    });
    let from_data_assignments = target_fields.iter().map(|f| {
        let Field {
            ident: field_name, ..
        } = f;
        quote! { self.data.#field_name = std::option::Option::Some(values.#field_name) }
    });
    quote! {
        #target_visibility struct #builder_name {
            data: #data_name,
            easing: std::option::Option<::mina::Easing>,
            normalized_time: f32,
        }

        impl #builder_name {
            fn new(normalized_time: f32) -> Self {
                Self {
                    normalized_time,
                    data: std::default::Default::default(),
                    easing: None,
                }
            }

            fn values_from(mut self, normalized_time: f32, values: &#remote_path) -> Self {
                #(#from_data_assignments);*;
                self
            }

            #(#setters)*
        }

        impl ::mina::KeyframeBuilder for #builder_name {
            type Data = #data_name;

            fn build(&self) -> ::mina::Keyframe<#data_name> {
                ::mina::Keyframe::new(
                    self.normalized_time, self.data.clone(), self.easing.clone())
            }

            fn easing(mut self, easing: ::mina::Easing) -> Self {
                self.easing = std::option::Option::Some(easing);
                self
            }
        }
    }
}

fn keyframe_struct(
    remote_name: &Ident,
    target_visibility: &Visibility,
    target_fields: &[&Field],
) -> TokenStream2 {
    let name = format_ident!("{remote_name}KeyframeData");
    let fields = target_fields.iter().map(|f| {
        let Field { ident, ty, .. } = f;
        quote! { #ident: std::option::Option<#ty> }
    });
    let values_struct = quote! {
        #[derive(std::clone::Clone, std::fmt::Debug, std::default::Default)]
        #target_visibility struct #name {
            #(#fields),*
        }
    };
    values_struct
}

fn timeline_builder_impl(remote_name: &Ident, target_fields: &[&Field]) -> TokenStream2 {
    let timeline_name = format_ident!("{remote_name}Timeline");
    let keyframe_data_name = format_ident!("{remote_name}KeyframeData");
    let sub_timeline_initializers = target_fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let sub_name = format_ident!("t_{field_name}");
        quote! {
            #sub_name: ::mina::SubTimeline::from_keyframes(
                &args.keyframes,
                std::default::Default::default(),
                |keyframe| keyframe.#field_name,
                args.default_easing.clone()
            )
        }
    });
    quote! {
        impl ::mina::TimelineBuilder<#timeline_name>
        for ::mina::TimelineConfiguration<#keyframe_data_name>
        {
            fn build(self) -> #timeline_name {
                let args = ::mina::TimelineBuilderArguments::from(self);
                #timeline_name {
                    timescale: args.timescale,
                    #(#sub_timeline_initializers),*,
                    boundary_times: args.boundary_times,
                }
            }
        }

        impl ::mina::TimelineOrBuilder<#timeline_name>
        for ::mina::TimelineConfiguration<#keyframe_data_name>
        {
            fn build(self) -> ::mina::MergedTimeline<#timeline_name> {
                ::mina::MergedTimeline::of([::mina::TimelineBuilder::build(self)])
            }
        }
    }
}

fn timeline_struct(
    remote_name: &Ident,
    target_visibility: &Visibility,
    target_fields: &[&Field],
) -> Result<TokenStream2> {
    let name = format_ident!("{remote_name}Timeline");
    let fields = target_fields
        .iter()
        .map(|f| {
            let Field { ident, ty, .. } = f;
            let name = format_ident!("t_{}", ident.as_ref().unwrap());
            Ok(quote! { #name: ::mina::SubTimeline<#ty> })
        })
        .collect::<Result<Vec<_>>>()?;
    let value_assignments = target_fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let sub_name = format_ident!("t_{field_name}");
        quote! {
            if let Some(#field_name) = self
                .#sub_name
                .value_at(normalized_time, frame_index, enable_start_override)
            {
                target.#field_name = #field_name;
            }
        }
    });
    let start_value_assignments = target_fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let sub_name = format_ident!("t_{field_name}");
        quote! {
            self.#sub_name.override_start_value(values.#field_name);
        }
    });
    let timeline_struct = quote! {
        #[derive(std::clone::Clone, std::fmt::Debug)]
        #target_visibility struct #name {
            boundary_times: std::vec::Vec<f32>,
            timescale: ::mina::TimeScale,
            #(#fields),*
        }

        impl ::mina::Timeline for #name {
            type Target = #remote_name;

            fn cycle_duration(&self) -> Option<f32> {
                Some(self.timescale.get_cycle_duration())
            }

            fn delay(&self) -> f32 {
                self.timescale.get_delay()
            }

            fn duration(&self) -> f32 {
                self.timescale.get_duration()
            }

            fn repeat(&self) -> Repeat {
                self.timescale.get_repeat()
            }

            fn start_with(&mut self, values: &Self::Target) {
                #(#start_value_assignments)*
            }

            fn update(&self, target: &mut Self::Target, time: f32) {
                let Some((normalized_time, frame_index, enable_start_override)) =
                    ::mina::prepare_frame(time, self.boundary_times.as_slice(), &self.timescale)
                else {
                    return;
                };
                #(#value_assignments)*
            }
        }

        impl ::mina::TimelineOrBuilder<#name> for #name {
            fn build(self) -> ::mina::MergedTimeline<#name> {
                ::mina::MergedTimeline::of([self])
            }
        }
    };
    Ok(timeline_struct)
}

#[cfg_attr(feature = "parse-debug", derive(Debug))]
struct AnimateAttributeInput {
    span: Span,
    name: Ident,
    _separator: Token![=],
    value: Lit,
}

impl Parse for AnimateAttributeInput {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            span: input.span(),
            name: input.parse()?,
            _separator: input.parse()?,
            value: input.parse()?,
        })
    }
}
