/// Attempt to parse variants in order until a match is found
macro_rules! parse_any {
    ($vis:vis enum $enum:ident {
        $(
            $variant:ident($ty:ty)
        ),*
        $(,)?
    }) => {
        $vis enum $enum<const WRITE: bool> {
            $(
                $variant($ty)
            ),*
        }

        impl<const WRITE: bool> ::syn::parse::Parse for $enum<WRITE> {
            fn parse(input: ::syn::parse::ParseStream<'_>) -> ::syn::Result<Self> {
                use $crate::parser::macros::RwMarker;
                $(if (<$ty as RwMarker>::READ == !WRITE || <$ty as RwMarker>::WRITE == WRITE) && <<$ty as $crate::parser::KeywordToken>::Token as ::syn::token::Token>::peek(input.cursor()) {
                    input.parse().map(Self::$variant)
                } else)* {
                    let mut error = String::from("expected one of: ");
                    $(
                        if <$ty as RwMarker>::READ == !WRITE || <$ty as RwMarker>::WRITE == WRITE {
                            error.push_str(<$ty as $crate::parser::KeywordToken>::display());
                            error.push_str(", ");
                        }
                    )*
                    error.truncate(error.len() - 2);
                    Err(input.error(error))
                }
            }
        }
    };
}

pub(super) use parse_any;

// The way this works sucks for a couple reasons which are not really worth
// dealing with right now, but maybe are worth dealing with in the future:
//
// 1. Using a separate enum just for parsing, instead of implementing parsing
// within a generated struct, shouldn’t really be necessary, but seemed to be
// the simplest to make everything work within the confines of the syn API.
// There is no way to get a `ParseStream` in syn other than to implement
// `syn::parse::Parse`, and that API return signature is `Result<Self>`, but the
// parser should to be able to return partial results instead (as it does now),
// so it’d be necessary to instead implement `Parse` for `PartialResult` and
// then go through an internal API that actually does parsing (and probably also
// reimplements other stuff like `Punctuated` since there would no longer be a
// type containing all the possible directives). It would be possible also to
// attach errors to the structs themselves, but it did not seem like the extra
// work to move non-fatal errors there was really worth the added effort since
// the current design was already written and functioning.
//
// 2. The variant-to-field mapping is awful. The `from` attributes should be
// taking types instead of idents, but can’t because then there would be no way
// to generate the enum variants. Variant names could be provided separately,
// but this would clutter the call sites for no particularly good reason—the
// types are normalised enough that it’s possible to just fill out the rest of
// the type here, even though it’s a nasty obfuscation that will confuse anyone
// that doesn’t look at what the macro is doing.
//
// So, you know… here be dragons, and I’m sorry in advance.
macro_rules! attr_struct {
    (
        #[from($attr_ty:ident)]
        $(#[$meta:meta])*
        $vis:vis struct $ident:ident {
        $(
            $(#[doc = $field_doc:literal])*
            $(#[cfg($($cfg_ident:tt)*)])?
            $(#[from($($field_rw:ident : $field_attr_id:ident),+)])?
            $field_vis:vis $field:ident : $field_ty:ty
        ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $ident {
            $(
                $(#[cfg($($cfg_ident)*)])?
                $(#[doc = $field_doc])*
                $field_vis $field: $field_ty,
            )+

            pub(crate) keyword_spans: Vec<proc_macro2::Span>,
        }

        impl<const WRITE: bool> $crate::parser::FromAttrs<$attr_ty<WRITE>> for $ident {
            fn try_set_attr(&mut self, attr: $attr_ty<WRITE>) -> ::syn::Result<()> {
                use crate::parser::KeywordToken;
                match attr {
                    $($(
                        $($attr_ty::$field_attr_id(value) => {
                            self.keyword_spans.push(value.keyword_span());
                            value.into_inner().try_set(&mut self.$field)
                        },)+
                    )?)+
                }
            }
        }

        $crate::parser::macros::parse_any! {
            $vis enum $attr_ty {
                $($(
                    $($field_attr_id($crate::parser::macros::$field_rw<$crate::parser::attrs::$field_attr_id>),)+
                )?)+
            }
        }
    }
}

pub(super) use attr_struct;

pub(super) trait RwMarker {
    const READ: bool;
    const WRITE: bool;
}

macro_rules! rw_marker {
    ($ident:ident, $read:literal, $write:literal) => {
        pub(crate) struct $ident<T>(T);

        impl<T> $ident<T> {
            pub(super) fn into_inner(self) -> T {
                self.0
            }
        }

        impl<T> RwMarker for $ident<T> {
            const READ: bool = $read;
            const WRITE: bool = $write;
        }

        impl<T: crate::parser::KeywordToken> crate::parser::KeywordToken for $ident<T> {
            type Token = T::Token;

            fn keyword_span(&self) -> proc_macro2::Span {
                T::keyword_span(&self.0)
            }
        }

        impl<T: syn::parse::Parse> syn::parse::Parse for $ident<T> {
            fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
                T::parse(input).map(Self)
            }
        }
    };
}

rw_marker!(RO, true, false);
rw_marker!(WO, false, true);
rw_marker!(RW, true, true);
