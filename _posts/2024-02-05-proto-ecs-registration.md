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

When you write a game engine, you usually want to allow users to create custom data types to implement behavior. And sometimes you need some sort of reflection system to collect some metadata about those new data types. Consider [Unreal Headers Tool](https://docs.unrealengine.com/5.0/en-US/unreal-header-tool-for-unreal-engine/) for example. In languages with limited reflection features like Rust and C++ this can be quite a chanllenge. In this article I will explain how we approached this problem using macros in Rust for my hobby engine, [proto-ecs]({% post_url 2024-01-04-proto-ecs-index-en %}).

In proto-ecs we decided to go for a **data oriented** design. Entities are just containers of data and systems. This means that entities require some way of specifying which components they need and how they are initialized. In Rust usually this problem is solved by manually adding data types in a project early on the program set up. See [this example from bevy engine](https://bevyengine.org/learn/quick-start/getting-started/ecs/#your-first-system):

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

* **[inventory](https://crates.io/crates/inventory)** : This crate implements a more general purpose version of what we're about to do with `ctor`, it allows you to register struct instances with a macro. This crate works for most cases, so it's very likely it fits your needs. 
* **[linkme](example-registration)** : This crate allows you to have static slices of elements that are gathered into a contiguous section in the binary. Basically allows you to have a pre-filled array of static items.

We didn't use those crates for `proto-ecs` because we needed a lot of macro code to register metadata and implement wiring boilerplate anyways, so ctor was a better fit for our use case. 

## Back to ctor

Now, for the next example we will implement a simple component registration macro similar to the one we 
use in `proto-ecs`. The idea is that:

1. We define a **component** as a struct
2. The component provides a **factory function** that knows how to create a component
3. Using **a macro**, we **register** the component by:
    1. Creating an entry in the component registry for this struct. A component registry is just a container of metadata about components
    2. Creating a wrapper function over the factory function that the component management system knows how to call

```rust
use crc32fast::hash;
use ctor::ctor;
use lazy_static::lazy_static;
use std::any::Any;
use std::sync::Mutex;

// 1
lazy_static! {
    static ref COMPONENT_REGISTERS: Mutex<Vec<RegisterEntry>> = Mutex::new(vec![]);
}

#[derive(Debug)]
struct RegisterEntry {
    name: String,
    name_crc: u32,
    factory: fn() -> Box<dyn Any>,
    id: u32
}
// 2
macro_rules! register_component {
    ($component:ident, $function:ident) => {
        #[ctor]
        fn add_to_register() {

            // 2.1
            fn factory_any() -> Box<dyn Any> {
                let component = $function();
                Box::new(component)
            }

            // 2.2
            let _ = COMPONENT_REGISTERS.lock().as_mut().and_then(|registry| {
                let id = registry.len() as u32;
                registry.push(RegisterEntry {
                name: stringify!($component).to_string(),
                name_crc: hash(stringify!($component).as_bytes()),
                factory: factory_any,
                id 
                });
                Ok(())
            });
        }
    };
}
// 3
struct MyComponent {
    name: String,
}
// 4
fn create_component() -> MyComponent {
    MyComponent {
        name: "Default Name".to_string(),
    }
}
// 5
register_component! {
    MyComponent,
    create_component
}
fn main() {
    let registry = COMPONENT_REGISTERS.lock().unwrap();

    // 6
    for entry in registry.iter() {
        let any_value = (entry.factory)();
        let component = any_value.downcast_ref::<MyComponent>().unwrap();

        // name: MyComponent, name_crc: 1359051788, id: 0
        println!("name: {}, name_crc: {}, id: {}", entry.name, entry.name_crc, entry.id);
        // Component name: Default Name
        println!("Component name: {}", component.name);
    }
}
```

Let's go step by step with this code:

1. We first create a registry, a **vector of entries**, of components. Each entry defined by some metadata about the component, in this case the name, a crc of the name that you could use for persistent storage, and the factory function. The registry is static so that it can be reached from any point. Also note that lazy static doesn't allows to have direct mutable references to anything, so you have to rely on some structure with [interior mutability](https://dhghomon.github.io/easy_rust/Chapter_41.html), in this case 
2. We define a macro that, given a struct identifier and a function identifier, writes a new function marked with `ctor`. This function will be ran automatically before main. In this function we:
    1. Define the wraper for the factory function
    2. Push a new entry for the component registration system, using the metadata we can compute from the macro.
3. Define a component by creating a struct type with some data
4. Define the factory function for that struct. Note that the factory function returns an instance of the struct.
5. We use the macro we defined earlier to register the component
6. Finally, in the main function we check that our component is actually registered without doing anything!

> Note that since the macro is defining a new function right were it's called, multiple macro invocations can generate errors with multiple components. It can also become anoying because we're generating functions that the user can call but he shouldn't. To fix this problem, you can use the following cool scoping trick: When writing the macro, you write the generated function like this: `const _ : () = {/*your generated function goes here*/};`. This way, the function exists but is not reachable outside the `= {...};` block. `ctor` will have no problem to find the function, and your users won't complain about all that garbage in their code suggestions.

This is the minimal setup you need to build a registration system, and works for simple cases just like this. However, things get a bit harder when we try to introduce dependencies between registered items. 

# Dependencies and execution order

Now consider the following use case: We want to create systems that *depend* on components. This way, if we add that system to some game entity, we know which components that entity must provide, and therefore we take useful actions like telling the user in advance that the entity is missing some required components, or even add them right away. For our puposes, we will consider that a function of type `(Vec<Box<dyn Any>>) -> ()` is a system.

We will need a way te add ids to components if we want to check in runtime which components an entity depends on. Let's create and implement a trait to help us with that:

```rust
// New dependency:
use std::cell::OnceCell;

// New trait:
pub trait HasID {
    fn get_id() -> u32;
    fn set_id(new_id : u32);
}

// Modify component registration
macro_rules! register_component {
    ($component:ident, $function:ident) => {
        #[ctor]
        fn add_to_component_registry() {
            // Add wraper over factory function...

            let _ = COMPONENT_REGISTRY.lock().as_mut().and_then(|registry| {
                let id = registry.len() as u32;
                // Push this component to the registry...
                $component::set_id(id); // New: Set id for this component
                Ok(())
            });
        }

        // New: Where we store the  actual id
        static COMPONENT_ID : Mutex<OnceCell<u32>> = Mutex::new(OnceCell::new());

        // New: Trait implementation
        impl HasID for $component
        {
            fn get_id() -> u32
            {
                *COMPONENT_ID.lock().unwrap().get().expect("ID not yet set")
            }

            fn set_id(new_id : u32)
            {
                let _ = COMPONENT_ID.lock().as_mut().map(
                        |id| { let _ = id.set(new_id).expect("ID already set"); }
                    );
            }
        }
    };
}
```

In this code we are just giving components the ability to have an ID.

1. We define a new Trait that provides the API to accessing the ID.
2. We add an id field to the `ComponentRegistryEntry` struct to store the id.
3. We implement the `HasID` trait automagically within the macro expansion. 
    1. Note that we store the ID in a static variable so that it's accessible anywhere when the function is called
    2. Also note that we use `OnceCell` so that the ID can only be modified once. Further `set` operations will crash the application.
    3. We use a **Mutex** as a simple way to share the ID between threads (which is demanded by the compiler), but you should probably use better synchronization mechanisms. For example, consider changing the Mutex for a [RwLock](https://doc.rust-lang.org/std/sync/struct.RwLock.html) and using [once_cell](https://crates.io/crates/once_cell) as the  `OnceCell` backend.

Now that components can provide an ID, we can create a system registration macro similar to the one we use for components:

```rust
lazy_static! {
    static ref SYSTEM_REGISTRY: Mutex<Vec<SystemRegistryEntry>> = Mutex::new(vec![]);
}

#[derive(Debug)]
struct SystemRegistryEntry {
    name: String,
    name_crc: u32,
    function: fn(Vec<Box<dyn Any>>) -> (),
    id: u32,
    dependencies: Vec<u32>
}

macro_rules! register_system {
    ($function:ident, dependencies = [$($dependency:ident),*]) => {
        #[ctor]
        fn add_to_system_registry() {
            let _ = SYSTEM_REGISTRY.lock().as_mut().and_then(|registry| {
                let dependencies : Vec<u32> = vec![$($dependency ::get_id() ),*];
                let id = registry.len() as u32;
                    registry.push(SystemRegistryEntry{
                    name: stringify!($function).to_string(),
                    name_crc: hash(stringify!($function).as_bytes()),
                    function: $function,
                    dependencies,
                    id 
                });
                Ok(())
            });
        }
    };
}

// A sample system
fn my_system (components : Vec<Box<dyn Any>>)
{
    let comp : &MyComponent = components[0].downcast_ref().unwrap();
    println!("My component is: {}", comp.name);
}
```

As you can see, we follow the same pattern we used for components, but we add the `dependencies` list that contains ids of all components required by this entity. For now we won't do anything fancy to keep the code simple, but you could do anything you want. For example, you could create a wrapper function over the system function to do type checking of the components list in runtime.  

Now we will register the system and the component with our new registration macros:

```rust
register_component! {
    MyComponent,
    create_component
}
register_system!(my_system, dependencies= [MyComponent]); // New
```

And we update the main properly to check our work:

```rust
fn main() {
    let comp_registry = COMPONENT_REGISTRY.lock().unwrap();
    for entry in comp_registry.iter() {
        let any_value = (entry.factory)();
        let component = any_value.downcast_ref::<MyComponent>().unwrap();
        // name: MyComponent, name_crc: 1359051788, id: 0
        println!("name: {}, name_crc: {}, id: {}", entry.name, entry.name_crc, entry.id);
        println!("Component name: {}", component.name);
    }

    // New
    let system_registry = SYSTEM_REGISTRY.lock().unwrap();
    for entry in system_registry.iter() {
        // System name: my_system, id: 0, dependencies: [0]
        println!("System name: {}, id: {}, dependencies: {:?}", entry.name, entry.id, entry.dependencies);
    }
}
```

Now everything should be going smooth until now. But let's see what happens if we change the registration order between our component and our system:

```rust
// Mind the registration order
register_system!(my_system, dependencies = [MyComponent]);
register_component! {
    MyComponent,
    create_component
}
```

If we rerun the code again, we get the following error:

```bash
thread '<unnamed>' panicked at src\main.rs:127:1:
ID not yet set
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
error: process didn't exit successfully: `target\debug\example-registration.exe` (exit code: 0xc0000409, STATUS_STACK_BUFFER_OVERRUN)
```

We get this error because we registered `my_system` function before `MyComponent`, and the registration function for `my_system` happened to be ran before the one used for `MyComponent`, and the ID for `MyComponent` was not yet configured. 

Now is when we remember why Rust didn't allow life before main in the first place, and in fact this is a very good reason why we shouldn't be doing this: span this problem over multiple files and we get an undebuggable mess. However, we still have one more trick to get away with our registration feature. 

See, the problem here is that we *implicitely* assume that components are loaded everytime we register a system, which cannot be ensured because ctor-marked functions execution order is not guaranteed. We have make sure that **functions that are marked with ctor can be called in any order**. 

Instead of executing functions that register systems and components, we will store them, and then calling them in the right order. The change we have to make is actually simple, we will just store a lambda that does everything we have been doing so far, and then just call it in the main before doing anything else.

```rust
// Component registration data types:
lazy_static! {
    static ref COMPONENT_REGISTRY: Mutex<Vec<ComponentRegistryEntry>> = Mutex::new(vec![]);
}

// New: We store registration functions in this vector
lazy_static! {
    static ref COMPONENT_REGISTRATION_FUNCTIONS: Mutex<Vec<fn() -> ()>> = Mutex::new(vec![]);
}

// Component Registration macro:
macro_rules! register_component {
    ($component:ident, $function:ident) => {
        #[ctor]
        fn add_to_component_registry() {
            fn function_to_add() {
                // Everything we used to do in this function before...
            }

            // New: Add this function to the functions-to-run list
            let _ = COMPONENT_REGISTRATION_FUNCTIONS.lock().as_mut().map(|functions| {
                functions.push(function_to_add);
            });
        }
    }

    // Trait implementation...
}

// System registration data types:
lazy_static! {
    static ref SYSTEM_REGISTRY: Mutex<Vec<SystemRegistryEntry>> = Mutex::new(vec![]);
}

// New: We store system registration functions in this vector
lazy_static! {
    static ref SYSTEM_REGISTRATION_FUNCTIONS: Mutex<Vec<fn() -> ()>> = Mutex::new(vec![]);
}

// System registration macro:
macro_rules! register_system {
    ($function:ident, dependencies = [$($dependency:ident),*]) => {
        #[ctor]
        fn add_to_system_registry() {
            fn function_to_add() {
                // Everything we used to do here...
            }

            // New: Add the function to the registration function list
            let _ = SYSTEM_REGISTRATION_FUNCTIONS.lock().as_mut().map(|functions| {
                functions.push(function_to_add);
            });
        }
    };
}

fn main() {
    // New: Register components and systems in the right order
    for comp_fn in COMPONENT_REGISTRATION_FUNCTIONS.lock().as_ref().unwrap().iter() {
        comp_fn();
    }
    for sys_fn in SYSTEM_REGISTRATION_FUNCTIONS.lock().as_ref().unwrap().iter() {
        sys_fn();
    }

    let comp_registry = COMPONENT_REGISTRY.lock().unwrap();
    for entry in comp_registry.iter() {
        let any_value = (entry.factory)();
        let component = any_value.downcast_ref::<MyComponent>().unwrap();
        // name: MyComponent, name_crc: 1359051788, id: 0
        println!("name: {}, name_crc: {}, id: {}", entry.name, entry.name_crc, entry.id);
        println!("Component name: {}", component.name);
    }

    let system_registry = SYSTEM_REGISTRY.lock().unwrap();
    for entry in system_registry.iter() {
        // System name: my_system, id: 0, dependencies: [0]
        println!("System name: {}, id: {}, dependencies: {:?}", entry.name, entry.id, entry.dependencies);
    }
}
```

In this code we just add static vectors where we store function pointers to the actual functions that perform the registration. In the ctor-function we define the registration function and then push them to the vector. Note that we are still using ctor, but the operations we mark with ctor are **independent of one another**. 

Finally, we run all the registration functions with an explicit order we can control. We said before that we didn't wanted a setup step in the main function, but this is one that we can afford because we only need a reference to the function list, we don't need a reference to each new data type!

# Final toughts

This is more or less how we solved the registration problem in proto-ecs. With this approach we can do the minimum amount of work needed to register data types throughout the project in a safe way, since the only work done is a vector push operation. We can also control the execution order of registration, so we have fine grained control over our app setup. Here are some additional things to consider:

* I highly encourage using [proc-macros](https://doc.rust-lang.org/reference/procedural-macros.html) instead of [macros-by-example](https://doc.rust-lang.org/reference/macros-by-example.html). We used macros-by-example here to simplify the resulting code but there are many things that are easier to do with proc-macro.
* The resulting code is **not multi-platform** by any mean, since `ctor` itself is not.
* All the reasons to avoid life-before-main is still valid, try to reduce the amount of work done with `ctor` as much as possible.

This was one of the most interesting parts of the development of proto-ecs, and the process of understanding how crates like `ctor` or `inventory` are implemented was a fun and educational experience. I hope you learn something from this article as well!