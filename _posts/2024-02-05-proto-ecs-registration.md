---
title: "[proto-ecs] Registering data types" 
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
register_component!(
    MyComponent,
    factory = create_my_component
)

// main.rs
fn main() {
    // Automagically get data to all registered data types, 
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
