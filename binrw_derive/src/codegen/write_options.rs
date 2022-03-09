use crate::parser::{write::Input, Assert, AssertionError, Map};
use proc_macro2::TokenStream;
use quote::quote;

mod r#struct;
use r#struct::generate_struct;

mod prelude;
mod struct_field;

mod r#enum;
use r#enum::{generate_data_enum, generate_unit_enum};

#[allow(clippy::wildcard_imports)]
use crate::codegen::sanitization::*;

pub(crate) fn generate(input: &Input, derive_input: &syn::DeriveInput) -> TokenStream {
    let name = Some(&derive_input.ident);
    let inner = match input.map() {
        Map::None => match input {
            Input::UnitStruct(_) => todo!(), //generate_unit_struct(input, name, None),
            Input::Struct(s) => generate_struct(input, name, s),
            Input::Enum(e) => generate_data_enum(input, name, e),
            Input::UnitOnlyEnum(e) => generate_unit_enum(input, name, e),
        },
        Map::Try(map) | Map::Map(map) => {
            let try_op = matches!(input.map(), Map::Try(_)).then(|| quote! { ? });
            let write_data = quote! {
                #WRITE_METHOD(
                    &((#map)(self) #try_op),
                    #WRITER,
                    #OPT,
                    ()
                )?;
            };

            let magic = input.magic();
            let endian = input.endian();
            prelude::PreludeGenerator::new(write_data, Some(input), name)
                .prefix_magic(magic)
                .prefix_endian(endian)
                .prefix_imports()
                .finish()
        }
    };

    //quote! {
    //    let #POS = #SEEK_TRAIT::stream_position(#READER)?;
    //    (|| {
    //        #inner
    //    })().or_else(|error| {
    //        #SEEK_TRAIT::seek(#READER, #SEEK_FROM::Start(#POS))?;
    //        Err(error)
    //    })
    //}

    quote! {
        let #POS = #SEEK_TRAIT::stream_position(#WRITER)?;
        #inner

        Ok(())
    }
}

fn get_assertions(assertions: &[Assert]) -> impl Iterator<Item = TokenStream> + '_ {
    assertions.iter().map(
        |Assert {
             condition,
             consequent,
         }| {
            let error_fn = match &consequent {
                Some(AssertionError::Message(message)) => {
                    quote! { #ASSERT_ERROR_FN::<_, fn() -> !>::Message(|| { #message }) }
                }
                Some(AssertionError::Error(error)) => {
                    quote! { #ASSERT_ERROR_FN::Error::<fn() -> &'static str, _>(|| { #error }) }
                }
                None => {
                    let condition = condition.to_string();
                    quote! { #ASSERT_ERROR_FN::Message::<_, fn() -> !>(|| { #condition }) }
                }
            };

            quote! {
                #ASSERT(#condition, #POS, #error_fn)?;
            }
        },
    )
}

fn get_map_err(pos: IdentStr) -> TokenStream {
    quote! {
        .map_err(|e| {
            #BIN_ERROR::Custom {
                pos: #pos,
                err: Box::new(e) as _,
            }
        })
    }
}
