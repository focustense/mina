extern crate proc_macro;

use proc_macro::TokenStream;

mod derive_animate;
mod fn_animator;

#[proc_macro]
pub fn animator(input: TokenStream) -> TokenStream {
    fn_animator::animator_impl(input)
}

#[proc_macro_derive(Animate, attributes(animate))]
pub fn derive_animate(input: TokenStream) -> TokenStream {
    derive_animate::animate_impl(input)
}
