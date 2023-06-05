use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, spanned::Spanned, Data, DeriveInput, Error, Field, Fields,
    Meta, Path, Result, Visibility,
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
        attrs: _attrs,
        vis,
    } = input;
    let Data::Struct(struct_data) = data else {
        return Err(Error::new(Span::call_site(), "derive(Animate) requires a struct type."));
    };
    let Fields::Named(fields) = struct_data.fields else {
        return Err(Error::new(
            struct_data.fields.span(),
            "derive(Animate) requires a struct with named fields."));
    };

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

    let builder_shortcuts = builder_shortcuts(&name);
    let values_struct = animator_values(&name, &vis, &anim_fields)?;
    let timeline_struct = timeline_struct(&name, &vis, &anim_fields)?;
    let timeline_builder_impl = timeline_builder_impl(&name, &anim_fields);
    let keyframe_struct = keyframe_struct(&name, &vis, &anim_fields);
    let keyframe_builder = keyframe_builder(&name, &vis, &anim_fields);
    let animate = quote! {
        #builder_shortcuts
        #values_struct
        #timeline_struct
        #timeline_builder_impl
        #keyframe_struct
        #keyframe_builder
    };

    Ok(animate)
}

fn animator_values(
    target_name: &Ident,
    target_visibility: &Visibility,
    target_fields: &[&Field],
) -> Result<TokenStream2> {
    let name = format_ident!("{target_name}AnimatorValues");
    // Recreate the fields to strip all attributes.
    let fields = target_fields.iter().map(|f| {
        let Field {
            ident: field_name,
            ty,
            ..
        } = f;
        quote! { #field_name: #ty }
    });
    let getters = target_fields
        .iter()
        .map(|f| field_getter(f, parse_quote! { pub }));
    let setters = target_fields
        .iter()
        .map(|f| field_setter(f, Visibility::Inherited));
    let mutators = target_fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        quote! {
            dest.#field_name = self.#field_name;
        }
    });
    let values_struct = quote! {
        #[derive(std::fmt::Debug, std::default::Default)]
        #target_visibility struct #name {
            #(#fields),*
        }

        impl #name {
            #(#getters)*

            #(#setters)*

            pub fn update(&self, dest: &mut #target_name) {
                #(#mutators);*
            }
        }
    };
    Ok(values_struct)
}

fn builder_shortcuts(target_name: &Ident) -> TokenStream2 {
    let builder_name = format_ident!("{target_name}KeyframeBuilder");
    let data_name = format_ident!("{target_name}KeyframeData");
    quote! {
        impl #target_name {
            pub fn keyframe(normalized_time: f32) -> #builder_name {
                #builder_name::new(normalized_time)
            }

            pub fn timeline() -> ::mina::TimelineConfiguration<#data_name> {
                ::mina::TimelineConfiguration::default()
            }
        }
    }
}

fn field_getter(field: &Field, visibility: Visibility) -> TokenStream2 {
    let Field {
        ref ident, ref ty, ..
    } = field;
    quote! {
        #visibility fn #ident(&self) -> #ty { self.#ident }
    }
}

fn field_setter(field: &Field, visibility: Visibility) -> TokenStream2 {
    let Field {
        ref ident, ref ty, ..
    } = field;
    let setter_name = Ident::new(
        &format!("set_{}", ident.as_ref().unwrap()),
        Span::call_site(),
    );
    quote! {
        #visibility fn #setter_name(&mut self, #ident: #ty) { self.#ident = #ident; }
    }
}

fn is_animatable(field: &Field) -> bool {
    field.attrs.iter().any(|attr| {
        let Meta::Path(ref path) = attr.meta else { return false; };
        is_simple_path(path, "animate")
    })
}

fn is_simple_path<'a>(path: &Path, name: impl Into<&'a str>) -> bool {
    path.segments.len() == 1
        && path.segments[0].arguments.is_none()
        && path.segments[0].ident == name.into()
}

fn keyframe_builder(
    target_name: &Ident,
    target_visibility: &Visibility,
    target_fields: &[&Field],
) -> TokenStream2 {
    let builder_name = format_ident!("{target_name}KeyframeBuilder");
    let data_name = format_ident!("{target_name}KeyframeData");
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

            #(#setters)*
        }

        impl ::mina_core::timeline::KeyframeBuilder for #builder_name {
            type Data = #data_name;

            fn build(&self) -> ::mina_core::timeline::Keyframe<#data_name> {
                ::mina_core::timeline::Keyframe::new(
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
    target_name: &Ident,
    target_visibility: &Visibility,
    target_fields: &[&Field],
) -> TokenStream2 {
    let name = format_ident!("{target_name}KeyframeData");
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

fn timeline_builder_impl(target_name: &Ident, target_fields: &[&Field]) -> TokenStream2 {
    let timeline_name = format_ident!("{target_name}Timeline");
    let values_name = format_ident!("{target_name}AnimatorValues");
    let keyframe_data_name = format_ident!("{target_name}KeyframeData");
    let sub_timeline_initializers = target_fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let sub_name = format_ident!("t_{field_name}");
        quote! {
            #sub_name: ::mina_core::timeline_helpers::SubTimeline::from_keyframes(
                &args.keyframes,
                defaults.#field_name,
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
                let args = ::mina_core::timeline::TimelineBuilderArguments::from(self);
                let defaults = #values_name::default();
                #timeline_name {
                    timescale: args.timescale,
                    #(#sub_timeline_initializers),*,
                    boundary_times: args.boundary_times,
                }
            }
        }
    }
}

fn timeline_struct(
    target_name: &Ident,
    target_visibility: &Visibility,
    target_fields: &[&Field],
) -> Result<TokenStream2> {
    let name = format_ident!("{target_name}Timeline");
    let values_name = format_ident!("{target_name}AnimatorValues");
    let fields = target_fields
        .iter()
        .map(|f| {
            let Field { ident, ty, .. } = f;
            let name = format_ident!("t_{}", ident.as_ref().unwrap());
            Ok(quote! { #name: ::mina_core::timeline_helpers::SubTimeline<#ty> })
        })
        .collect::<Result<Vec<_>>>()?;
    let value_assignments = target_fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let sub_name = format_ident!("t_{field_name}");
        let setter_name = format_ident!("set_{field_name}");
        quote! {
            if let Some(#field_name) = self.#sub_name.value_at(normalized_time, frame_index) {
                values.#setter_name(#field_name);
            }
        }
    });
    let timeline_struct = quote! {
        #target_visibility struct #name {
            boundary_times: std::vec::Vec<f32>,
            timescale: ::mina_core::time_scale::TimeScale,
            #(#fields),*
        }

        impl ::mina::Timeline for #name {
            type Values = #values_name;

            fn values_at(&self, time: f32) -> Self::Values {
                let mut values = Self::Values::default();
                let Some((normalized_time, frame_index)) = ::mina_core::timeline::prepare_frame(
                    time, self.boundary_times.as_slice(), &self.timescale
                ) else {
                    return values;
                };
                #(#value_assignments)*
                values
            }
        }
    };
    Ok(timeline_struct)
}
