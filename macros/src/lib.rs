extern crate proc_macro;

use proc_macro::TokenStream;

mod animate;
mod animator;

#[proc_macro]
pub fn animator(input: TokenStream) -> TokenStream {
    animator::animator_impl(input)
}

#[proc_macro_derive(Animate, attributes(animate))]
pub fn derive_animate(input: TokenStream) -> TokenStream {
    animate::animate_impl(input)
}