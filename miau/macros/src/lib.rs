use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{DeriveInput, ExprPath, parse_macro_input};
use quote::{quote, format_ident};

fn ctor() -> TokenStream2 {
  quote! {
    #[cfg_attr(target_os = "linux", link_section = ".ctors")]
    #[cfg_attr(target_os = "macos", link_section = "__DATA,__mod_init_func")]
    #[cfg_attr(target_os = "windows", link_section = ".CRT$XCU")]
  }
}

#[proc_macro_attribute]
pub fn component(_: TokenStream, input: TokenStream) -> TokenStream {
  let input2 = TokenStream2::from(input.clone());
  let DeriveInput { ident, .. } = parse_macro_input!(input);
  let c = format_ident!("_{}_INIT", ident);
  let ctor = ctor();
  quote! {
    #[allow(non_upper_case_globals)]
    #ctor
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

#[proc_macro_attribute]
pub fn asset(args: TokenStream, input: TokenStream) -> TokenStream {
  let loader: ExprPath = parse_macro_input!(args);
  let input2 = TokenStream2::from(input.clone());
  let DeriveInput { ident, .. } = parse_macro_input!(input);
  let c = format_ident!("_{}_LOAD", ident);
  let ctor = ctor();
  quote! {
    #[allow(non_upper_case_globals)]
    #ctor
    static #c: extern fn() = {
      fn loader(data: &[u8]) -> crate::Result<::std::rc::Rc<dyn ::std::any::Any>> {
        #loader(data).map(|a| ::std::rc::Rc::new(a) as _)
      }

      extern fn i () {
        unsafe { crate::assets::ASSET_LOADERS.insert(::std::any::TypeId::of::<#ident>(), crate::assets::AssetLoader {loader, assets: vec![]}); }
      }
      i
    };
    #input2
  }
  .into()
}
