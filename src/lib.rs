/*!
Has this ever happened to you?

```rust
#[derive(Debug, Default, PartialEq, Eq)]
struct Inner {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Mid {
    a: Inner,
    b: Inner,
    c: Inner,
    d: Inner
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Outer {
    mid1: Mid,
    mid2: Mid,
    mid3: Mid,
    mid4: Mid,
}

fn build_outer() -> Outer {
    Outer {
        mid1: Mid {
            a: Inner {
                x: 10,
                ..Default::default()  // :D
            },
            b: Inner {
                y: 10,
                ..Default::default()  // :)
            },
            ..Default::default()  // :|
        },
        mid2: Mid {
            b: Inner {
                z: 10,
                ..Default::default()  // :/
            },
            ..Default::default()  // :(
        },
        ..Default::default()  // >:(
    }
}
```

Wouldn't it be nice if you could omit all the tedious `..Default::default()`
calls when building deeply nested struct literals? Now you can! With
`autodefault`, it's never been easier to build up a large struct literal
for your tests, [bevy](https://bevyengine.org/) components, or anything else
you might need!. Simply tag any function with the `#[autodefault]` attribute
and let us handle the rest:

```rust
use autodefault::autodefault;
# #[derive(Debug, Default, PartialEq, Eq)]
# struct Inner {
#     x: i32,
#     y: i32,
#     z: i32,
# }
#
# #[derive(Debug, Default, PartialEq, Eq)]
# struct Mid {
#     a: Inner,
#     b: Inner,
#     c: Inner,
#     d: Inner
# }
#
# #[derive(Debug, Default, PartialEq, Eq)]
# struct Outer {
#     mid1: Mid,
#     mid2: Mid,
#     mid3: Mid,
#     mid4: Mid,
# }
#
# fn build_outer() -> Outer {
#     Outer {
#         mid1: Mid {
#             a: Inner {
#                 x: 10,
#                 ..Default::default()  // :D
#             },
#             b: Inner {
#                 y: 10,
#                 ..Default::default()  // :)
#             },
#             ..Default::default()  // :|
#         },
#         mid2: Mid {
#             b: Inner {
#                 z: 10,
#                 ..Default::default()  // :/
#             },
#             ..Default::default()  // :(
#         },
#         ..Default::default()  // >:(
#     }
# }

#[autodefault]
fn build_outer_simple() -> Outer {
    Outer {
        mid1: Mid {
            a: Inner { x: 10 },
            b: Inner { y: 10 },
        },
        mid2: Mid {
            b: Inner { z: 10 },
        }
    }
}  // :O

assert_eq!(build_outer(), build_outer_simple())
```

It's never been easier!

# What it's actually doing

When applied to a function, the `#[autodefault]` will scan the body of the
function for all struct literals that don't already have a `..rest` trailing
initializer and insert a `..Default::default()`. It will do this unconditionally
for all struct literals, regardless of whether they actually implement
[`Default`], so be sure to refactor into helper functions as necessary:

```compile_fail
use autodefault::autodefault;

struct NoDefault {
    x: i32,
    y: i32,
    z: i32,
}

#[autodefault]
fn nope() {
    let _nope = NoDefault { x: 10 };
}
```
*/

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::{
    parse2, parse_quote,
    visit_mut::{visit_expr_struct_mut, VisitMut},
    ExprStruct, ItemFn,
};

struct AutodefaultVisitor;

impl VisitMut for AutodefaultVisitor {
    fn visit_expr_struct_mut(&mut self, struct_expr: &mut ExprStruct) {
        visit_expr_struct_mut(self, struct_expr);

        // Make sure fields have trailing comma
        if !struct_expr.fields.empty_or_trailing() {
            struct_expr.fields.push_punct(parse_quote! {,});
        }

        // Add `..Default::default()`
        if struct_expr.dot2_token.is_none() && struct_expr.rest.is_none() {
            struct_expr.dot2_token = Some(parse_quote! {..});
            struct_expr.rest = Some(Box::new(parse_quote! {
                ::core::default::Default::default()
            }));
        }
    }
}

fn autodefault_impl(item: TokenStream2) -> TokenStream2 {
    let mut item: ItemFn = parse2(item).unwrap();
    let mut visitor = AutodefaultVisitor;
    visitor.visit_item_fn_mut(&mut item);
    item.into_token_stream()
}

/// Modify a function such that all struct literals include `..Default::default()`.
///
/// See [module][crate] docs for details.
#[proc_macro_attribute]
pub fn autodefault(_attr: TokenStream, item: TokenStream) -> TokenStream {
    autodefault_impl(item.into()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream as TokenStream2;
    use quote::quote;

    #[test]
    fn it_works() {
        let input = quote! {
            fn demo () {
                let x = Foo {a: 10, b: 10};
            }
        };
        let output: TokenStream2 = autodefault_impl(input);

        assert_eq!(
            format!("{:?}", output),
            format!(
                "{:?}",
                quote! {
                    fn demo () {
                        let x = Foo {a: 10, b: 10, ..::core::default::Default::default()};
                    }
                }
            ),
        )
    }

    #[test]
    fn trailing_comma() {
        let input = quote! {
            fn demo () {
                let x = Foo {a: 10, b: 10, };
            }
        };
        let output: TokenStream2 = autodefault_impl(input);

        assert_eq!(
            format!("{:?}", output),
            format!(
                "{:?}",
                quote! {
                    fn demo () {
                        let x = Foo {a: 10, b: 10, ..::core::default::Default::default()};
                    }
                }
            ),
        )
    }

    #[test]
    fn empty_struct() {
        let input = quote! {
            fn demo () {
                let x = Foo {};
            }
        };
        let output: TokenStream2 = autodefault_impl(input);

        assert_eq!(
            format!("{:?}", output),
            format!(
                "{:?}",
                quote! {
                    fn demo () {
                        let x = Foo {..::core::default::Default::default()};
                    }
                }
            ),
        )
    }

    #[test]
    fn existing_spread() {
        let input = quote! {
            fn demo () {
                let x = Foo {a: 10, b: 10, ..foo()};
            }
        };
        let output: TokenStream2 = autodefault_impl(input);

        assert_eq!(
            format!("{:?}", output),
            format!(
                "{:?}",
                quote! {
                    fn demo () {
                        let x = Foo {a: 10, b: 10, ..foo()};
                    }
                }
            ),
        )
    }
}
