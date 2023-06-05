extern crate proc_macro;

use proc_macro::TokenStream;

mod animate;

#[proc_macro_derive(Animate, attributes(animate))]
pub fn derive_animate(input: TokenStream) -> TokenStream {
    animate::animate_impl(input)
}