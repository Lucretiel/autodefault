/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

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
`autodefault`, it's never been easier to build up a large struct expression
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
function for all struct expressions that don't already have a `..rest` trailing
initializer and insert a `..Default::default()`. It will do this unconditionally
for all struct expressions, regardless of whether they actually implement
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
# Filtering `Default` insertions

If you only want to add `..Default::default()`  to some of the structs in your
function, `autodefault` supports filtering by type name:

```
use autodefault::autodefault;

#[derive(Default)]
struct HasDefault {
    a: i32,
    b: i32,
    c: i32,
    d: i32,
}

struct NoDefault1 {
    a: HasDefault,
}

struct NoDefault2 {
    a: NoDefault1,
}

#[autodefault(except(NoDefault1, NoDefault2))]
fn example1() {
    let _data = NoDefault2 { a: NoDefault1 { a: HasDefault {} } };
}

#[autodefault(only(HasDefault))]
fn example2() {
    let _data = NoDefault2 { a: NoDefault1 { a: HasDefault {} } };
}
```

# Other behaviors

`autodefault` will not descend into nested item definitions; if you nest an
`fn` item inside another `fn`, you'll need to tag the inner function with
`autodefault` again.

```
use autodefault::autodefault;

#[derive(Default)]
struct HasDefault {
    x: i32
}

struct NoDefault {
    x: i32
}

#[autodefault]
fn outer() {
    let _x = HasDefault {};

    fn inner() {
        let _x = NoDefault {x: 10};
    }
}
```

*/

use std::collections::HashSet;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::{
    parenthesized,
    parse::Parse,
    parse2, parse_quote,
    punctuated::Punctuated,
    visit_mut::{visit_expr_struct_mut, VisitMut},
    ExprStruct, Ident, Item, Token,
};

#[derive(Debug)]
enum Rule {
    Only,
    Except,
}

#[derive(Debug)]
enum Rules {
    All,
    Only(HashSet<Ident>),
    Except(HashSet<Ident>),
}

impl Parse for Rules {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Rules::All);
        }

        let rule_ident: Ident = input.parse()?;
        let rule = if rule_ident == "except" {
            Rule::Except
        } else if rule_ident == "only" {
            Rule::Only
        } else {
            return Err(syn::Error::new(
                rule_ident.span(),
                "Expected 'except' or 'only'",
            ));
        };

        let content;
        let _parens = parenthesized!(content in input);

        let rules: Punctuated<Ident, Token![,]> = Punctuated::parse_terminated(&content)?;
        let rules = rules.into_iter().collect();

        Ok(match rule {
            Rule::Only => Rules::Only(rules),
            Rule::Except => Rules::Except(rules),
        })
    }
}

struct AutodefaultVisitor {
    rules: Rules,
}

impl AutodefaultVisitor {}

impl VisitMut for AutodefaultVisitor {
    fn visit_expr_struct_mut(&mut self, struct_expr: &mut ExprStruct) {
        visit_expr_struct_mut(self, struct_expr);

        let struct_ident = &struct_expr.path.segments.last().unwrap().ident;

        match &self.rules {
            Rules::Only(allow_list) if !allow_list.contains(struct_ident) => return,
            Rules::Except(deny_list) if deny_list.contains(struct_ident) => return,
            _ => {}
        }

        // Add `..Default::default()` to structs that don't have a ..rest
        // initializer
        if struct_expr.dot2_token.is_none() && struct_expr.rest.is_none() {
            // Make sure fields have trailing comma
            if !struct_expr.fields.empty_or_trailing() {
                struct_expr.fields.push_punct(parse_quote! {,});
            }

            // Add the ..Default::default()
            struct_expr.dot2_token = Some(parse_quote! {..});
            struct_expr.rest = Some(Box::new(parse_quote! {
                ::core::default::Default::default()
            }));
        }
    }

    fn visit_item_mut(&mut self, _: &mut Item) {}
}

fn autodefault_impl(attr: TokenStream2, item: TokenStream2) -> TokenStream2 {
    let rules = match parse2(attr) {
        Ok(rules) => rules,
        Err(err) => return err.into_compile_error(),
    };

    let mut item = match parse2(item) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    let mut visitor = AutodefaultVisitor { rules };

    visitor.visit_item_fn_mut(&mut item);
    item.into_token_stream()
}

/// Modify a function such that some or all struct expressions include
/// `..Default::default()`.
///
/// See [module][crate] docs for details.
#[proc_macro_attribute]
pub fn autodefault(attr: TokenStream, item: TokenStream) -> TokenStream {
    autodefault_impl(attr.into(), item.into()).into()
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
        let output: TokenStream2 = autodefault_impl(TokenStream2::new(), input);

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
        let output: TokenStream2 = autodefault_impl(TokenStream2::new(), input);

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
        let output: TokenStream2 = autodefault_impl(TokenStream2::new(), input);

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
        let output: TokenStream2 = autodefault_impl(TokenStream2::new(), input);

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

    #[test]
    fn except() {
        let output = autodefault_impl(
            quote! { except(Ignore1, Ignore2) },
            quote! {
                fn demo() {
                    let a = Ignore1 {};
                    let b = Ignore2 {};
                    let c = Default1 {};
                    let d = Default2 {};
                }
            },
        );

        assert_eq!(
            format!("{:?}", output),
            format!(
                "{:?}",
                quote! {
                    fn demo() {
                        let a = Ignore1 {};
                        let b = Ignore2 {};
                        let c = Default1 {..::core::default::Default::default()};
                        let d = Default2 {..::core::default::Default::default()};
                    }
                }
            )
        )
    }

    #[test]
    fn only() {
        let output = autodefault_impl(
            quote! { only(Default1, Default2) },
            quote! {
                fn demo() {
                    let a = Ignore1 {};
                    let b = Ignore2 {};
                    let c = Default1 {};
                    let d = Default2 {};
                }
            },
        );

        assert_eq!(
            format!("{:?}", output),
            format!(
                "{:?}",
                quote! {
                    fn demo() {
                        let a = Ignore1 {};
                        let b = Ignore2 {};
                        let c = Default1 {..::core::default::Default::default()};
                        let d = Default2 {..::core::default::Default::default()};
                    }
                }
            )
        )
    }

    #[test]
    fn inner_item() {
        let input = quote! {
            fn demo () {
                let x = Foo {a: 10, b: 10};

                fn inner () {
                    let x = Foo {a: 10, b: 10};
                }
            }
        };
        let output: TokenStream2 = autodefault_impl(TokenStream2::new(), input);

        assert_eq!(
            format!("{:?}", output),
            format!(
                "{:?}",
                quote! {
                    fn demo () {
                        let x = Foo {a: 10, b: 10, ..::core::default::Default::default()};

                        fn inner () {
                            let x = Foo {a: 10, b: 10};
                        }
                    }
                }
            ),
        )
    }
}
