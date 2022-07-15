#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use codec::{Decode, Encode, HasCompact};
use scale_info::TypeInfo;
struct Example {
    x: u128,
    y: u64,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Example {
    #[inline]
    fn clone(&self) -> Example {
        match *self {
            Example {
                x: ref __self_0_0,
                y: ref __self_0_1,
            } => Example {
                x: ::core::clone::Clone::clone(&(*__self_0_0)),
                y: ::core::clone::Clone::clone(&(*__self_0_1)),
            },
        }
    }
}
impl ::core::marker::StructuralPartialEq for Example {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Example {
    #[inline]
    fn eq(&self, other: &Example) -> bool {
        match *other {
            Example {
                x: ref __self_1_0,
                y: ref __self_1_1,
            } => match *self {
                Example {
                    x: ref __self_0_0,
                    y: ref __self_0_1,
                } => (*__self_0_0) == (*__self_1_0) && (*__self_0_1) == (*__self_1_1),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Example) -> bool {
        match *other {
            Example {
                x: ref __self_1_0,
                y: ref __self_1_1,
            } => match *self {
                Example {
                    x: ref __self_0_0,
                    y: ref __self_0_1,
                } => (*__self_0_0) != (*__self_1_0) || (*__self_0_1) != (*__self_1_1),
            },
        }
    }
}
#[allow(deprecated)]
const _: () = {
    #[automatically_derived]
    impl ::codec::Encode for Example {
        fn encode_to<__CodecOutputEdqy: ::codec::Output + ?::core::marker::Sized>(
            &self,
            __codec_dest_edqy: &mut __CodecOutputEdqy,
        ) {
            ::codec::Encode::encode_to(&self.x, __codec_dest_edqy);
            ::codec::Encode::encode_to(&self.y, __codec_dest_edqy);
        }
    }
    #[automatically_derived]
    impl ::codec::EncodeLike for Example {}
};
#[allow(deprecated)]
const _: () = {
    #[automatically_derived]
    impl ::codec::Decode for Example {
        fn decode<__CodecInputEdqy: ::codec::Input>(
            __codec_input_edqy: &mut __CodecInputEdqy,
        ) -> ::core::result::Result<Self, ::codec::Error> {
            ::core::result::Result::Ok(Example {
                x: {
                    let __codec_res_edqy = <u128 as ::codec::Decode>::decode(__codec_input_edqy);
                    match __codec_res_edqy {
                        ::core::result::Result::Err(e) => {
                            return ::core::result::Result::Err(
                                e.chain("Could not decode `Example::x`"),
                            )
                        }
                        ::core::result::Result::Ok(__codec_res_edqy) => __codec_res_edqy,
                    }
                },
                y: {
                    let __codec_res_edqy = <u64 as ::codec::Decode>::decode(__codec_input_edqy);
                    match __codec_res_edqy {
                        ::core::result::Result::Err(e) => {
                            return ::core::result::Result::Err(
                                e.chain("Could not decode `Example::y`"),
                            )
                        }
                        ::core::result::Result::Ok(__codec_res_edqy) => __codec_res_edqy,
                    }
                },
            })
        }
    }
};
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::default::Default for Example {
    #[inline]
    fn default() -> Example {
        Example {
            x: ::core::default::Default::default(),
            y: ::core::default::Default::default(),
        }
    }
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    impl ::scale_info::TypeInfo for Example {
        type Identity = Self;
        fn type_info() -> ::scale_info::Type {
            ::scale_info::Type::builder()
                .path(::scale_info::Path::new("Example", "binary_interface"))
                .type_params(::alloc::vec::Vec::new())
                .docs(&[])
                .composite(
                    ::scale_info::build::Fields::named()
                        .field(|f| f.ty::<u128>().name("x").type_name("u128").docs(&[]))
                        .field(|f| f.ty::<u64>().name("y").type_name("u64").docs(&[])),
                )
        }
    };
};
fn main() {
    let example = Example { x: 1, y: 2 };
    let example_encoded = example.encode();
    {
        ::std::io::_print(::core::fmt::Arguments::new_v1(&["Hello, world!\n"], &[]));
    };
    {
        ::std::io::_print(::core::fmt::Arguments::new_v1(
            &["Encoded value: ", "\n"],
            &[::core::fmt::ArgumentV1::new_debug(&example_encoded)],
        ));
    };
}
