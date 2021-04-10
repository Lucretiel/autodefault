[![Crates.io](https://img.shields.io/crates/v/autodefault)](https://crates.io/crates/autodefault) [![Crates.io](https://img.shields.io/crates/l/autodefault)](https://crates.io/crates/autodefault) [![docs.rs](https://img.shields.io/docsrs/autodefault)](http://docs.rs/autodefault)

# autodefault

A library that automatically inserts `..Default::default()` for you.

## The pitch

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

Wouldn't it be nice if you could omit all the tedious `..Default::default()` calls when building deeply nested struct literals? Now you can! With `autodefault`, it's never been easier to build up a large struct literal for your tests, [bevy](https://bevyengine.org/) components, or anything else you might need!. Simply tag any function with the `#[autodefault]` attribute and let us handle the rest:

```rust
use autodefault::autodefault;

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

It's never been easier! Check out the [reference documentation](http://docs.rs/autodefault) for more details.
