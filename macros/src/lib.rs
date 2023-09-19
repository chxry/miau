use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{DeriveInput, parse_macro_input};
use quote::{quote, format_ident};

#[proc_macro_attribute]
pub fn component(_: TokenStream, input: TokenStream) -> TokenStream {
  let input2 = TokenStream2::from(input.clone());
  let DeriveInput { ident, .. } = parse_macro_input!(input);
  let c = format_ident!("_{}_INIT", ident);
  quote! {
    #[allow(non_upper_case_globals)]
    #[cfg_attr(target_os = "linux", link_section = ".ctors")]
    #[cfg_attr(target_os = "macos", link_section = "__DATA,__mod_init_func")]
    #[cfg_attr(target_os = "windows", link_section = ".CRT$XCU")]
    static #c: extern fn() = {
      use ::std::any::{Any, TypeId};
      use ::std::cell::{RefCell, Ref};

      fn ser(c: Ref<dyn Any>) -> Ref<dyn ::erased_serde::Serialize> {
        Ref::map(c, |c| unsafe {c.downcast_ref_unchecked::<#ident>()})
      }

      fn de(de: &mut dyn ::erased_serde::Deserializer) -> Box<RefCell<dyn Any>> {
        Box::new(RefCell::new(::erased_serde::deserialize::<#ident>(de).unwrap()))
      }

      extern fn i() {
        unsafe { crate::ecs::COMPONENTS.insert(TypeId::of::<#ident>(), (ser, de)); }
      }
      i
    };
    #input2
  }
  .into()
}
