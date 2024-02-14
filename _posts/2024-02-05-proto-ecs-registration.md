---
title: "[proto-ecs] Data types registration" 
layout: post
author: Luis Diaz
tags: [Rust, Game Engine, proto-ecs]
miniature: https://upload.wikimedia.org/wikipedia/commons/thumb/d/d5/Rust_programming_language_black_logo.svg/2048px-Rust_programming_language_black_logo.svg.png
description: Sometimes you might want to register component types in your game engine to implement things like serialization, data-driven entity descriptions and so on, but Rust has very limited reflection features. In this article I explain how we solved components auto-registration for proto-ecs using <strong>ctor</strong>.
language: en
repo: 
es_version: false
---

In [proto-ecs]({% post_url 2024-01-04-proto-ecs-index-en %}) we decided to go for a **data oriented** design. Entities are just containers of data and systems. This means that entities require some way of specifying which components they need and how they are initialized. In Rust usually this problem is solved by manually adding data types in a project early on the program set up. See [this example from bevy engine](https://bevyengine.org/learn/quick-start/getting-started/ecs/#your-first-system):

```rust
use bevy::prelude::*;

fn hello_world() {
    println!("hello world!");
}

fn main() {
    App::new()
        .add_systems(Update, hello_world)
        .run();
}
```

However we wanted data types to be registered within the engine without user input (consider something like Unreal or Unity) and that all the required wiring logic was handled within the file where data types are defined. Consider something like this:

```rust
// my_component.rs
struct MyComponent {
    name : String,
}
fn create_my_component() -> MyComponent {
    MyComponent{name: "default name".to_string()}
}

// Register some metadata related to this component
register_component!(
    MyComponent,
    factory = create_my_component
)

// main.rs
fn main() {
    // Automagically get metadata for all registered data types, 
    for comp_data in get_all_components() {
        // Call the create_my_component function that was previously registered
        let component = (comp_data.factory)();
    }
}
```

1. We first define a component as a struct with the data it holds
2. Then we create a function that implements any required logic to create a new component (a factory function)
3. We register the component and the factory function
4. In the main function we _magically_ have access to all defined components and their factory functions, plus any other metadata we might need. 

This system plays particularly well with serialization, we can just create a persistent id on registration for each component and map it to the right factory function when restoring an instance. 

Now the core problem is, where in the program execution are components collected, if not in the main function in a setup step?

# The Problem

We want to somehow collect all components defined by users of the engine and register some metadata and their factory functions in some global variable accessible anywhere in the application. However, [Rust has very limited support for reflection](https://stackoverflow.com/questions/36416773/how-does-rust-implement-reflection), and we don't want users write them up in some central file.

Additionally, notice that since we don't have references to component types, we can't actually just use the output of the factory function defined by the user, what type would it have?. We need to wrap that function inside another one that returns some [trait object](https://doc.rust-lang.org/book/ch17-02-trait-objects.html) we can actually type.

We could make our own main function, but how would we get references to all user-defined components? We could use macros to write a function that registers new components to a global component registry, but how would the main function get all references to those new functions?

We could even register components on a global variable on a proc-macro when we are processing the `register_component!` macro and then add them with another macro in the main function when all components are registered, but [proc macro execution order is not guaranteed](https://github.com/rust-lang/reference/issues/578) (and global variables in proc macros are a [bad idea](https://stackoverflow.com/questions/52910783/is-it-possible-to-store-state-within-rusts-procedural-macros) in general).

We could try to use [`lazy_static!`](https://docs.rs/lazy_static/latest/lazy_static/) to register data types but we have a similar problem that when we tried to create registration functions with macros, the main function would require a way to reference all those lazy-static variables to call them at the least once. 

So, if we can't do nothing from our main function, _is there something before main?_

# Life before main

In other languages sometimes you need to run some code before everything else in the appliation. This code runs even before the first line of code in the `main` function, take [static initializers](https://en.cppreference.com/w/cpp/language/initialization) in C++ for example; This is what we call **life before main**. In Rust, [we don't have life before main](https://doc.rust-lang.org/1.4.0/complement-design-faq.html#there-is-no-life-before-or-after-main-(no-static-ctors/dtors)), and there are [very good reasons](http://yosefk.com/c++fqa/ctors.html#fqa-10.12) to avoid them, and some of the use cases of life-before-main are handled pretty well by [lazy-static initialization](https://crates.io/crates/lazy_static), which is implemented by creating a variable that gets initialized at some point after main when the static variables are accessed for the first time. However the plugin-registration style of static initialization we try to implement still requires actual life-before-main.

This could be easier if Rust supported some sintax or feature to hook init funtions to some explicitly specified point in main, and there's an interesting discussion around this topic in this post titled [From “life before main” to “common life in main”](https://internals.rust-lang.org/t/from-life-before-main-to-common-life-in-main/16006). For now, we will stick to a non-portable life-before-main solution: [ctor](https://crates.io/crates/ctor).

# Running code before main

**ctor** is a crate that implements a macro that you can add to any function to force it to run before main:

```rust
// From ctor documentation:
static INITED: AtomicBool = AtomicBool::new(false);

#[ctor]
fn foo() {
    INITED.store(true, Ordering::SeqCst);
}

fn main() {
    assert!(INITED.load(Ordering::SeqCst));
}
```

* In this code, we create a static variable `INITED` initialized to `false`.
* We also create a function `foo` that sets `INITED` to `true`
* Since `foo` is marked with `#[ctor]`, it will be called before anything we do in the main function.

The main idea here is to use `#[ctor]` along with a macro to push some data into some container whenever we want to register a new datatype. 

## A quick look at some alternatives
Before we continue, also consider the following crates:

* **[inventory](https://crates.io/crates/inventory)** : This crate implements a more general purpose version of what we're about to do with `ctor`, it allows you to register data instances with a macro. This crate works for most cases, so it's very likely it fits your needs. 
* **[linkme](example-registration)** : This crate allows you to have static slices of elements that are gathered into a contiguous section in the binary. Basically allows you to have a pre-filled array of static items.

We didn't use those crates for `proto-ecs` because we needed a lot of macro code to register metadata and implement wiring boilerplate anyways, so ctor was a better fit for our use case. 

## Back to ctor

Now, for the next example we will implement a simple component registration macro similar to the one we 
use in `proto-ecs`. The idea is that:

1. We define a **component** as a struct
2. The component provides a **factory function** that knows how to create a component
3. Using **a macro**, we **register** the component by:
    1. Creating an entry in the component registry for this struct. A component registry is just a container of metadata about containers
    2. Creating a wrapper function over the factory function that the component management system knows how to call

```rust
use crc32fast::hash;
use ctor::ctor;
use lazy_static::lazy_static;
use std::any::Any;
use std::sync::Mutex;

lazy_static! {
    static ref COMPONENT_REGISTERS: Mutex<Vec<RegisterEntry>> = Mutex::new(vec![]);
}

#[derive(Debug)]
struct RegisterEntry {
    name: String,
    name_crc: u32,
    factory: fn() -> Box<dyn Any>,
}

macro_rules! register_component {
    ($component:ident, $function:ident) => {
        #[ctor]
        fn add_to_register() {
            fn factory_any() -> Box<dyn Any> {
                let component = $function();
                Box::new(component)
            }
            COMPONENT_REGISTERS.lock().unwrap().push(RegisterEntry {
                name: stringify!($component).to_string(),
                name_crc: hash(stringify!($component).as_bytes()),
                factory: factory_any,
            });
        }
    };
}

struct MyComponent {
    name: String,
}

fn create_component() -> MyComponent {
    MyComponent {
        name: "Default Name".to_string(),
    }
}

register_component! {
    MyComponent,
    create_component
}

fn main() {
    let registry = COMPONENT_REGISTERS.lock().unwrap();
    for entry in registry.iter() {
        let any_value = (entry.factory)();
        let component = any_value.downcast_ref::<MyComponent>().unwrap();

        // name: MyComponent, name_crc: 1359051788
        println!("name: {}, name_crc: {}", entry.name, entry.name_crc);
        // Component name: Default Name
        println!("Component name: {}", component.name);
    }
}
```

Let's go step by step with this code:

1. We first create a registry (vector of entries) of components. Each entry defined by some metadata about the component, in this case the name, a crc of the name that you could use for persistent storage, and the factory function. The registry is static so that it can be reached from any point. 
2. We define a macro that, given a struct identifier and a function identifier, writes a new function marked with `ctor`. This function will be ran automatically before main. In this function we:
    1. Define the wraper for the factory function
    2. Push a new entry for the component registration system, using the metadata we can compute from the macro.
3. Define a component by creating a struct type with some data
4. Define the factory function for that struct. Note that the factory function returns an instance of the struct.
5. We use the macro we defined earlier to register the component
6. Finally, in the main function we check that our component is actually registered without doing anything!

> Note that since the macro is defining a new function right were it's called, multiple macro invocations can generate errors with multiple errors. It can also become anoying because we're generating functions that the user can call but he shouldn't. To fix this problem, you can use the following cool scoping trick: When writing the macro, you write the generated function like this: `const _ : () = {/*your generated function goes here*/};`. This way, the function exists but is not reachable outside the `= {...};` block. `ctor` will have no problem to find the function, and your users won't complain about all that garbage in their code suggestions.

This is the minimal setup you need to build a registration system, and works for simple cases just like this. However, things get a bit harder when we try to introduce dependencies between registered items. 

