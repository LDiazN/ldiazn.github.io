---
title: "[proto-ecs] Entity Allocator" 
layout: post
author: Luis Diaz
tags: [Rust, Game Engine, proto-ecs]
miniature: https://upload.wikimedia.org/wikipedia/commons/thumb/d/d5/Rust_programming_language_black_logo.svg/2048px-Rust_programming_language_black_logo.svg.png
description: A thorough explanation of how we implemented entity allocation in Rust. We start with simple Generational Indices, and we explore several interesting Rust features to implement a complete entity allocator!
language: en
repo: https://github.com/LDiazN/example-allocators
es_version: false
---

When you write a game engine, one of the most important tasks of the engine is **managing entities' lifetime:** allocation, destruction, registration, and general bookkeping. Creating new entities can be expensive, especially if more memory has to be allocated. In this article, I will explain how we implemented the **entity allocator** we use in **proto-ecs,** how we compared multiple approaches, and which one we ended up choosing.

We will start by creating the simplest allocator using generational indices to ground up the concepts we need to start reasoning about different strategies, and we will define better allocators from there. Finally, no informed decision is complete without some benchmarking. We will use a basic setup with Criterion to test our allocators and choose wisely.

Don’t expect this to be the best possible implementation for a memory allocator, many considerations and implementation details come into play in each game or application. This is just the thought process we used to reach our current implementation in **proto-ecs**, and I hope it helps you to understand the basic concepts and find a path to your ideal implementation.

Also note that this article is focused on an _entity_ allocation, not general _memory_ allocation, like an arena allocator for example. As a reference, you can check [this implementation](https://github.com/pcwalton/offset-allocator) in Rust of Sebastian Aaltonen's **[Offset Allocator](https://github.com/sebbbi/OffsetAllocator)**, a simple and efficient arena allocator. 

# Entity Allocation 101

There are many considerations to keep in mind when designing an entity allocator, but it is essentially a _data structure_ that:

1. **Manages memory allocations:**

    1. Requests **more memory** from the OS when it’s needed

    2. **Reuses memory** whenever possible

    3. **Deallocates memory** when there’s too much unnecessary memory allocated

2. **Provides access** to the allocated entity, through a handle or pointer. This access has to be as fast as possible since state manipulation for entities is most certainly the most common operation.

3. **Identifies entities:** Provides the ability to differentiate between entities

In this article, we will focus more on the last two features, but the first one is not too hard to consider once we have a concrete implementation.

A useful pattern we can use to implement this data structure is **generational indices.** This pattern solves the problem of distinguishing different “reincarnations” of the same entity. Let’s say we define an array of entities as our memory pool, and the handle is an index in that array, we can encounter the following scenario:

- We store a handle to entity **i** in many other entities.

- At some point, **i** gets deleted, marking the position **i** in our entity pool array as available.

- Later, a new entity is created, and our memory allocator decides that it can use the available position **i** for this entity

- Now, all previous entities that referenced our first version of **i** will think that **i** was never deleted

There are other strategies to solve this problem. For example, you can use an **[observer pattern](https://en.wikipedia.org/wiki/Observer_pattern)** to detect when an entity gets deleted, but this can be error-prone, and keeping every list of observers consistent can be challenging, slow, and hard to debug.

Also note that this problem is similar to the **[ABA problem](https://en.wikipedia.org/wiki/ABA_problem),** but without the need for parallelism.

# Generational indices

The main idea behind generational indices is to have an array or container where each entry is the data you store. Those entries also have a **generation number** that will always increment, and it helps you to know if two indices point to the same entity even if the entry was reallocated at some point. We also have a pool of available indices that we use to choose the next position. So, to sum up, the main components we need are:

- An array to store data, along with a **generation number** for each entry

- A **queue of available entries** within the array

- A **pointer or handle** we give to users to get a reference to the data later on

We also have to define every operation we need our data structure to perform. For now, let’s just consider the most basic ones:

- `new` returns a pointer or handle to a new entity memory segment you can use to store your new entity

- `free` expects a pointer or handle generated by `new`, and it will free the associated memory and push the pointer into the "available" queue

- `is_live` expects a pointer or handle and returns a bool indicating if the related entity is still alive. Aka, if `free` was called at some point over that pointer

The following is a simple implementation we can use as a baseline to derive our efficient implementations later. It’s based on [Katherine West’s Talk on ECS and data representations in games for RustConf 2018](https://kyren.github.io/2018/09/14/rustconf-talk.html), and it’s more of a generational index allocator rather than a memory or entity allocator:

```rust

#[derive(Debug, PartialEq, Default)]
pub struct GenerationalIndices
{
    indices : Vec<u32>, // Generations. Indices are specified by the array position
    pub free : VecDeque<usize>
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct GenerationalIndex
{
    index : usize,
    generation : u32
}

impl GenerationalIndices
{
    pub fn new(&mut self) -> GenerationalIndex
    {
        if self.free.is_empty()
        {
            let next_index = self.indices.len();
            self.indices.push(0);
            return GenerationalIndex{index: next_index, generation: 0};
        }
        let index = self.free.pop_front().unwrap();
        let generation = self.indices[index];
        GenerationalIndex { index, generation}
    }

    #[inline(always)]
    pub fn is_live(&self, index: &GenerationalIndex) -> bool
    {
        index.generation == self.indices[index.index]
    }
    pub fn free(&mut self, index:&GenerationalIndex)
    {
        if ! self.is_live(&index)
        {
            return; // Report an error or something
        }

        self.free.push_back(index.index);
        self.indices[index.index] += 1;
    }
}
```

- The **free** list of pointers is implemented as a `VecDeque` to efficiently take from the start of the list and push to the end. Note that you need to implement the free list in a FIFO structure (queue) instead of LIFO one (stack) because when performing multiple allocations and deallocations, you can deplete the generation counter of the first few elements that always go in and out of the free list.

- The **indices** array stores the generation counter for each index

- `new` creates a new generational index by taking one from the free list. If not available, the indices vector capacity is increased. Also, note how the **generation** stored in the **indices** array is either the generation of a living entity or the next generation available for an entity when the index is in the free list

- `is_live` compares the generation of the input handle with the one currently stored in the indices array. If they match, the entity is alive. Otherwise, it was freed at some point. Note that it doesn’t matter if the entry is currently being used, the generation number will still say that the old entity was freed.

- `free` will return the index into the free list, increasing its generation counter so that further calls to `is_live`  return false for this pointer

This example illustrates how to allocate indices and how we can use generation numbers to check if an entity is dead or alive, but we still don't address memory access and lifetime. It turns out that this can be a challenging task in Rust due to the memory ownership and borrowing rules.

# Allocating memory with generational indices
To understand why Rust's memory model can be a problem with generational indices, consider the following generational index allocator implementation:

```rust
#[derive(Debug, Default)]
pub struct GenerationalArrayEntry<T>
{
    item : Option<T>,
    generation : u32
}

#[derive(Debug, Default)]
pub struct GenerationalIndexArray<T>
{
    elements : Vec<GenerationalArrayEntry<T>>,
    free: VecDeque<usize>
}

impl<T> GenerationalIndexArray<T>
{
    // New: the `new` function has to receive the value of the element to
    // assign to the allocated entry
    pub fn new(&mut self, element : T) -> GenerationalIndex
    {
        // Similar to the previous implementation, but you have to 
        // add `element` to the stored array of elements
    }

    // New: Get an immutable reference to the entry
    pub fn get(&self, index: &GenerationalIndex) -> Option<&T>
    {
        if !self.is_live(index)
        {
            return None;
        }

        return self.elements[index.get_index()].item.as_ref();
    }

    // New: Get a mutable reference to the entry
    pub fn get_mut(&mut self, index: &GenerationalIndex) -> Option<&mut T>
    {
        if !self.is_live(index)
        {
            return None;
        }

        return self.elements[index.get_index()].item.as_mut();
    }

    pub fn free(&mut self, index:&GenerationalIndex) { ... }

    pub fn is_live(&self, index: &GenerationalIndex) -> bool {...}
}

```

The `new` , `free`, and `get` functions can be easily implemented similarly with this structure, setting each entry's `item` field to `None` when they are free, and to `Some(value)` when they are allocated. Note that `new` now requires an additional argument, the initial value of the allocated entry. We *move* the ownership to the allocator because we don't want another reference to the same entity lying around there.

Also note that `get` functions can tell if the entity is alive or not, solving the use-after-free problem we had before.

The problem with this implementation is that, due to Rust's ownership rules, now we can't use multiple mutable references at the same time using the `get_mut` function multiple times, which is a big problem when we want to implement operations between entities. Consider the following code: 

```rust
#[derive(Debug, Default)]
struct Entity
{
id : GenerationalIndex,
name : String,
is_active : bool
}

let mut generational_array = GenerationalIndexArray::<Entity>::default();
let e1 = generational_array.new(Entity{name: "e1".to_string(), is_active: false, id: GenerationalIndex::default()});
let e2 = generational_array.new(Entity{name: "e2".to_string(), is_active: false, id: GenerationalIndex::default()});
let entity1 = generational_array.get_mut(&e1).unwrap();
let entity2 = generational_array.get_mut(&e2).unwrap(); // Error!
entity1.is_active = true;
entity2.name = true;
```

This code will fail to compile with the following error

```
error[E0499]: cannot borrow `generational_array` as mutable more than once at a time
  --> src\tests.rs:78:27
   |
77 | let entity1 = generational_array.get_mut(&e1).unwrap();
   | ------------------ first mutable borrow occurs here
78 | let entity2 = generational_array.get_mut(&e2).unwrap();
   | ^^^^^^^^^^^^^^^^^^ second mutable borrow occurs here
79 | entity1.is_active = true;
   | ------------------------ first borrow later used here

For more information about this error, try `rustc --explain E0499`.
error: could not compile `example_allocators` (lib test) due to previous error
warning: build failed, waiting for other jobs to finish...
```

In this case, the borrow checker is complaining because we have two mutable references to memory owned by the `generational_array` variable. 

If you think about it, it makes sense to provide access to many mutable references to entries within the allocator, because the **allocator couldn't care less about the current state of the memory it manages**, it only cares about which entries are free, and the generation for each entry. The problem is that Rust doesn't know about this! 

# RefCell to the rescue

The borrow checker won't allow us to have multiple references to a memory segment owned by the memory allocator, even though we know that the referenced segments are disjoint.

Since this is a memory allocator, what we need is:

* Many references to multiple segments of the internal array

* Mutable access to those references to update entity state

This can be achieved in safe Rust using [RefCell](https://doc.rust-lang.org/std/cell/index.html). A `RefCell` Is a **container** that allows you to have multiple references to a memory segment and mutate it using **[interior mutability](https://doc.rust-lang.org/reference/interior-mutability.html)** 

The borrowing rules in Rust say that you can have as many references as you want (aliases), but only one mutating reference at the time (inherited mutability). The trick with `RefCell` is that you take as many immutable references to elements within the array (`RefCell<T>`) and each reference implements the function `borrow_mut(&self) -> &mut T` (note the `&self` qualifier, denoting an immutable reference to `self`). This function will allow us to mutate the contained value `T`, using **interior mutability**. 

Let's see how this works:

```rust
// New version of array entry, use RefCell<T> instead of just T
pub struct GenerationalArrayEntryCell<T>
{
    item : Option<RefCell<T>>,
    generation : u32
}
```

So, a simple version of our previous allocator using this implementation would look like this:

```rust
impl<T> GenerationalIndexArrayCell<T>
{
    pub fn new(&mut self, element : T) -> GenerationalIndex
    {
        ...
    }
    #[inline(always)]
    pub fn is_live(&self, index: &GenerationalIndex) -> bool
    {
        ...
    }
    pub fn free(&mut self, index:&GenerationalIndex)
    {
        ...
    }
    pub fn get(&self, index: &GenerationalIndex) -> Option<&RefCell<T>>
    {
        // Check if this entity is alive...
        // New: Clone the shared reference (Rc) to create a new reference
        // to the same memory
        return (*self.elements[index.get_index()].item).as_ref();
    }
} 
```

Whenever we want to use a value, we do:

```rust
let mut allocator : GenerationalIndexArrayCell<Entity>= GenerationalIndexArrayCell::default();
let genid1 = allocator.new(Entity{
    name: "e1".to_string(),
    _is_active: false,
    _id: GenerationalIndex::default()}
);
let genid2 = allocator.new(Entity{
    name: "e2".to_string(),
    _is_active: false,
    _id: GenerationalIndex::default()}
);

let entity1 = allocator.get(&genid1).unwrap();
let entity2 = allocator.get(&genid2).unwrap();
// No compile errors! :)
entity1.borrow_mut()._is_active = true;
entity2.borrow_mut()._is_active = true;
```

# What about resizing?

You might have noticed that we are using a **vector** to store our entities, and this is good for performance because the data locality might keep cache misses in check. However, there is a case where our current implementation is inefficient, and that's when you **allocate too many entities**, especially if you don't know how many you might need. If you have to allocate many entities at once, you might have to **request several resize operations** for the vector (implicitly with each `push`). Since the actual data is located within the array, each array entry might be big and expensive to copy into a new memory segment. 

> Another problem that's not so common in Rust but is worth being aware of, is the fact that each resize might **invalidate pointers to positions within the array**. Using raw pointers is not common in Rust but if you are writing a game engine, that's probably not the craziest thing you are doing. 

If you want cheaper resize operations to handle these cases, you can change the `Option<RefCell<T>>` in the array entry to `Box<Option<RefCell<T>>`. This will allocate objects in the heap and store the pointer, and the `RefCell` will allow you to use the same trick to have many mutable references to positions within the same allocator array.

# An alternative approach with... Placement new?

Up until now, we have a functional implementation that you could use for many use cases. In this section, we will implement another version that is functionally the same as the one we already have but uses an interesting feature that will help us later when we try to implement a useful (and very unsafe) trick to implement our pointer type. 

Consider our current entry type:

```rust
pub struct GenerationalArrayEntryCell<T>
{
    generation : u32
    item : Option<RefCell<T>>,
}
```

In this type, we use the `Option<...>` part to `drop` and `init`ialize values. Notice how we set `item` to `None` when we free an element? We're essentially calling the destructor for that item. Assigning `None` also allows you to specify that this entry is in an invalid state (because it was deallocated with `free`). However, we don't need that part, as the initialized state is determined by the presence of this entry in the free list. 

So, instead of using `Option` to implement destruction and initialization, we will use a different approach. We will allocate **uninitialized memory** and construct and destruct entries there. 

The problem is that Rust doesn't allow you to specify the memory address where a value will be stored. Rust relies on the compiler being able to find the final memory location where an object should be constructed and optimize out unnecessary copies. Consider the following code as an example:

```rust
fn main() {
    let x = Box::new([[0u64; 1_000_000_000]]);
}
```

If you run this program with `cargo run`, it will crash with a stack overflow, because the internal array content overflows the stack. It is constructed in the stack, and then moved into the `Box`. 

On the other hand, `cargo run --release` runs with no problem, because the compiler can optimize unnecessary moves and writes the memory content where it should end. 

This is an extreme example but it shows that sometimes it's hard to control how objects are allocated in memory. In C++, you would usually solve this problem by allocating a block of memory beforehand and filling it with [**placement new**](https://stackoverflow.com/questions/222557/what-uses-are-there-for-placement-new).  

Placement new is a C++ feature that allows you to construct an object in a memory segment that's already allocated. This is something useful in cases like ours where we want to manage our program's memory ourselves, we can still construct objects like always but without worrying about requesting and freeing memory unnecessarily. 

However, Rust doesn't provide an equivalent language feature to placement new. This makes sense if you think about it because being able to place a new object on some pre-existent block of memory implies that you can have **uninitialized memory**, but the rust compiler **does not allow uninitialized variables**, and relies heavily on that assumption to implement optimizations and features. But in cases like ours, we can use `MaybeUninit`!

 [`MaybeUninit` ](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html) is a Rust type provided by the standard library to implement something _similar_ to placement new in C++. It allows you to have **uninitialized blocks of memory**. It tells the compiler that the internal memory is not initialized, turning off any optimization that relies on that assumption. 

You declare a `MaybeUninit` variable as follows: 

```rust

let x = MaybeUninit::<i32>::uninit();

```

Now `x` is of type `MaybeUninit<i32>`, and **it's guaranteed to have the same layout as an `i32`**. We can also use this property to always keep our objects in the same memory location and closer to each other!

Finally, we can implement an allocator using `MaybeUninit`:

```rust
#[derive(Default)]
pub struct GIABoxUninit<T>
where
    T: Default,
{
    entries: Vec<GIABoxUninitEntry<T>>,
    free: Vec<usize>,
}

pub type Generation = u32;

pub struct GIABoxUninitEntry<T> {
    generation: Generation,
    ptr: Box<MaybeUninit<T>>,
}
```

In this implementation, the `GIABoxUninitEntry` replaces the `Option` part in the previous one to specify the memory segment where the item will be stored. Let's see how we implement the `new` and `free` functions:   

```rust
 
pub fn new(&mut self, element : T) -> GenerationalIndex {
    if self.free.is_empty() {
        // Construct a new entry
        let mut new_entry = InPlaceAllocEntry {
            mem: MaybeUninit::<T>::uninit(),
            generation: 0,
        };
        let new_entry_index = self.entries.len();
        // Initialize it since it will be retrieved from this function
        new_entry.ptr.write(element);
        // Add it to the current list of entries
        self.entries.push(new_entry);
        return GenerationalIndex {
            index: new_entry_index,
            generation: 0,
        };
    }
    let next_free = self.free.pop().unwrap();
    let entry = &mut self.entries[next_free];
    // Initialize entry, don't return uninitialized memory
    entry.ptr.write(element);
    return GenerationalIndex {
        index: next_free,
        generation: entry.generation,
    };
}
```

Here we use `write` to store the element's memory within the allocator, instead of filling the entry with `Some(element)`. Note that `MaybeUninit::write` takes ownership of the argument. 

```rust
pub fn free(&mut self, index: &GenerationalIndex) {
    if !self.is_live(index) {
        panic!("Trying to free already unused index");
    }
    let index = index.index;
    self.free.push(index);
    let entry: &mut GIABoxUninitEntry<T> = &mut self.entries[index];
    entry.generation += 1;
    unsafe {
        entry.ptr.assume_init_drop();
    }
}
```

And we reached the first `unsafe` in this article. As for the `free` function we use `assume_init_drop` to call the `drop` function of the data that was stored in that entry. This operation is unsafe because the `MaybeUninit` can't ensure that the underlying memory is initialized, so calling the `drop` method could result in panic or undefined behavior. We know that it's initialized because it's alive, but this is something we have to ensure with our own logic. 

```rust
pub fn get(&self, index: &GenerationalIndex) -> Option<&RefCell<T>> {
    if !self.is_live(index) {
        return None;
    }
    let entry = &mut self.entries[index.index];
    return unsafe {
        Some(self.entries[index.index].ptr.assume_init_ref())
    };
}
```

And finally, in the get function, we can just ask for a mutable reference to the internal memory. Similarly to the previous cases, in the `get` function we use `assume_init` because we know that the entry is initialized (is `live`).

So we have gained... Nothing yet, it's just a refactor introducing unsafe calls. However, this approach will prove helpful later!

# Improving ergonomics with raw pointers

Now that we have some solid implementations, let's check how we use this code in action!

```rust
// Our dummy entity type
#[derive(Default)]
struct Entity
{
    id: usize, 
    is_active: bool,
    name: String
}

// Constructing the allocator and creating an entity
let mut gbu = GIABoxUninit::<Entity>::default();
let entity_handle = gbu.new(Entity::default());

// Initializing the entity:
let entity_ref = gbu.get(&entity_handle);
assert!(entity_ref.is_some());
let mut entity_ref = entity_ref.unwrap().borrow_mut();
entity_ref.id = 42;
entity_ref.name = "test1".to_owned();
entity_ref.is_active = true;
```

- Constructing the allocator is not hard, we just call the constructor once at the start of our program, and then we use that same instance everywhere.

- Creating an entity is also easy, we just use the new function that takes the ownership of a new entity we pass as argument.

- However, using an entity implies:

    - Getting a reference to the `RefCell` from the handle and the allocator

    - Checking if the entity is valid with `is_some`, but you can jump to unwrap if you don't care about crashing the app on an invalid pointer

    - getting a mutable reference to the actual entity from the `RefCell`

    - Finally using the entity

Using an entity is not only cumbersome, but we might run into borrow-checking issues if we want to free it:

```rust
let mut allocator= GIABoxUninit::<Entity>::default();
let entity_handle = bgu.new(Entity::default());
// Initializing the entity:
let entity_ref = allocator.get(&entity_handle); // immutable borrow of "allocator"
assert!(entity_ref.is_some());
let mut entity_ref = entity_ref.unwrap().borrow_mut();
allocator.free(&entity_handle) // ERROR: Mutable Borrow of "allocator"
```

If we would like to do something like this for whatever reason, we would be forced to scope the entity reference to prevent borrowing issues:

```rust
let mut allocator= GIABoxUninit::<Entity>::default();
let entity_handle = bgu.new(Entity::default());
{
    let entity_ref =allocator.get(&entity_handle); // inmutable borrow of "allocator"
    assert!(entity_ref.is_some());
    let mut entity_ref = entity_ref.unwrap().borrow_mut();
    // Do something with entity_ref...
}

allocator.free(&entity_handle) // OK: previous borrow is out of scope
```

So... yeah, not particularly comfortable to use. In the next section, we will work on better usability for our entity allocators. 

## Improving handle usability

Notice that the allocator manages the lifetime of stored objects, with `new` and `free`, but it doesn't care about what is being done with that memory. Therefore, we can afford us a bit more flexibility if we allow **unrestricted access to that memory through pointers**.

Consider the following implementation:

```rust
#[derive(Debug, Default)]
pub struct InPlaceAllocator<T>
where
    T: Default,
{
    entries: Vec<InPlaceAllocEntry<T>>,
    free: Vec<usize>,
}

#[derive(Debug)]
struct InPlaceAllocEntry<T> {
    value: RefCell<MaybeUninit<T>>,
    generation: Generation,
}
```

Notice how the type of `value` in `InPlaceAllocEntry` is `RefCell<MaybeUninit<T>>`, meaning that values are stored contiguously in memory. 

Now the implementation:

```rust
impl<T> InPlaceAllocator<T>
{
    pub fn get(&self, index: &GenerationalIndex) -> &mut T {
        debug_assert!(
            self.is_live(index),
            "Trying to retrieve uninitialized memory"
        );
        let entry = &self.entries[index.index];
        return unsafe { entry.value.borrow_mut().as_mut_ptr().as_mut().unwrap() };
    }
}
```

- `free`, `new`, and `is_live` are similar to the previous versions

- In `get`, we introduce an unsafe call to get a raw pointer to the `MaybeUninit`'s internal memory, and then we convert it to a mutable reference

- Notice how `RefCell` is important in this case, allowing us to get a mutable reference to something behind an immutable reference

- We removed the `Option<...>` from the return type to further improve the ergonomics of this API, but you can keep it if it fits better your usage patterns

With this simple change, we also get an interesting property: The borrow checker can now help us prevent a few errors:

```rust
// Create an allocator and a handle
let mut inplace_alloc = InPlaceAllocator::<entity>::default();
let entity_handle = inplace_alloc.new(entity::default());
// Inmutable borrow of inplace_alloc
let entity = inplace_alloc.get(&entity_handle);
//Try to get a mutable borrow of inplace_alloc
inplace_alloc.free(entity_handle);
```

In this example, we get a reference to the entity in a single call, but if we try to free the underlying entity, the compiler will complain, because of borrow checking issues: We try to get a mutable reference with `free` to the allocator, which was immutably borrowed when we got our entity reference.

A possible danger with this approach is that you can have **multiple mutable references to the same entity** without realizing it, the compiler can't save you here:

```rust
let mut inplace_alloc = InPlaceAllocator::<entity>::default();
let entity_handle = inplace_alloc.new(entity::default());
let entity_ref1 = inplace_alloc.get(&entity_handle);
let entity_ref2 = inplace_alloc.get(&entity_handle);
// Compiles with no problem, you can use both references as you want
```

You have to be careful with how you allow access to handles if you want your system to be safe!

## A basic pointer type

In this section, we will use a slightly different approach that will allow us to have our custom pointer type, so we don't even need a reference to the original allocator:     

```rust
#[derive(Default)]
pub struct BoxAllocator<T> {
    entries: Vec<Box<Entry<T>>>,
    free: Vec<*mut Entry<T>>,
}

pub struct Entry<T> {
    generation: Generation,
    value: MaybeUninit<T>
}
```

Each entity is stored in an `Entry` with a `generation` field as metadata, and a `value` field as the actual entity. Those entries are **stored within a Box** because we want the entries to remain in the heap. Specifically, we need to prevent resizes of the `entries` vector in the allocator from messing with pointers. If we store a raw pointer to a position within the vector, a resize could reallocate the entry to another memory address and our pointer would be invalidated. This problem could be solved with a list-of-arrays data structure instead of a vector, but we won't go there in this article. 

Notice that the type of `value` in the `Entry` struct is just `MaybeUninit<T>`, there's no `RefCell` as before. We do this because we want to allow unrestricted access to the internal memory. Since this is an allocator, the allocator doesn't care about what is being done with the memory it stores, it only cares about what memory segments are free or not. 

As previously mentioned, ensuring memory consistency is a responsibility of the system using the allocator!

On the other hand, our pointer type will be like this:

```rust
pub struct EntityPtr<T> {
    generation: Generation,
    ptr: *mut Entry<T>, // super unsafe raw pointer!
}
```

Where the `ptr` value corresponds to the internal memory address within the `Box<Entry<T>>` in the `Allocator`. It's quite important to prevent the user from constructing an `EntityPtr` by themselves, you should restrict the access to this type such that the only way to get a pointer is from the allocator:

```rust
impl<T> BoxAllocator<T> {
    pub fn new(&mut self, element: T) -> EntityPtr<T> {
        if self.free.is_empty() {
            // Construct a new entry
            let mut new_entry = Box::new(Entry {
                generation: 0,
                value: MaybeUninit::<T>::uninit(),
            });
            let new_entry_index = self.entries.len();
            // initialize
            new_entry.value.write(element);
            self.entries.push(new_entry);
            return EntityPtr{
                ptr: &mut *self.entries[new_entry_index] as *mut Entry<T>,
                generation: 0,
            };
        }
        let next_free = self.free.pop().unwrap();
        // Initialize entry, don't return uninitialized memory
        unsafe{(*next_free).value.write(element)};
        let generation = unsafe {
            (*next_free).generation
        };
        return EntityPtr{
            ptr: next_free,
            generation: generation
        }
    }
    pub fn free(&mut self, ptr: &EntityPtr<T>) {
        debug_assert!(ptr.is_live(), "Trying to double-free a pointer");
        self.free.push(ptr.ptr);
        unsafe {
           (*ptr.ptr).generation += 1;
           (*ptr.ptr).value.assume_init_drop();
        }
    }
}

```

Note that we removed the `get` and `is_live` functions from the allocator, this functions will be implemented by the pointer type:

```rust
impl<T> EntityPtr<T> {
    #[inline(always)]
    pub fn is_live(&self) -> bool {
        return self.generation == unsafe {(*self.ptr).generation}
    }
}
impl <T> Deref for EntityPtr<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        debug_assert!(self.is_live(), "Trying to deref free pointer");
        return unsafe {(*self.ptr).value.assume_init_ref()}
    }
}

impl <T> DerefMut for EntityPtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(self.is_live(), "Trying to deref free pointer");
        return unsafe {(*self.ptr).value.assume_init_mut()}
    }
}
```

- We implement `is_live` from `EntityPtr`, where we compare the generation within this pointer to the generation within the underlying `EntityEntry`. 

- To turn our type into a pointer, we have to implement `Deref` and `DerefMut`

- We also use a `debug_assert` to check for invalid access when we are developing our system

And finally, we can use our pointer more ergonomically:

```rust
let mut allocator = allocator_with_pointer::Allocator::<Entity>::default();
let mut entity1 = allocator.new(Entity::default());
let mut entity2 = allocator.new(Entity::default());
entity1.id = 42;
entity1.is_active = true;
entity2.id = 69;
entity2.name = "Second Entity".to_owned();
```

No `get`, no `borrow`, no `unwrap`, no borrowing issues. Just access your data immediately!

## Safety assumptions

The safety of this approach relies heavily on the following assumptions:

1. Pointers can be accessed and written to at any moment. **The system using this allocator will be responsible for ensuring data consistency**

2. **The allocator will live more than any other pointer**, because when it's dropped, all pointer access turns into undefined behavior 

3. Since the signature of the `Defer` and `DeferMut` force you to return a reference to the target type, you can't return an `Option` when trying to retrieve the entity. Therefore, **access to invalidated pointers will either cause undefined behavior** if you don't check them as we did, or **crash your application. You choose**!

Also note that the pointer type we defined is big, if the generation is 32-bit long, then you have a pointer pointer with size 32 + 64 = 96 bits long (plus padding)!

# Final thoughts

To choose an allocator style for our system, we had to consider many properties for each and how they interact with `proto-ecs`. In this section, we will list the properties of each type of allocator.

We will also focus on the last two allocators we implemented in this article: `InPlaceAllocator` and `BoxAllocator`. 

|           | **`InPlaceAllocator`**                                                 | **`BoxAllocator`** | 
| **Cache** | ✅ Cache usage is efficient since all entities are stored contiguously | ❌ Entities are scattered through memory |
| **Addition** | ❌ Since entities are stored in place within the internal vector, addition can be expensive if a resize is triggered | ✅ Although a resize might be triggered, the copied objects will be just pointers, not actual entities that might be big |
| **Retrieval** | ✅ Access is implemented as array indexing which is quite fast | ✅ Access is implemented as straight-up memory dereferencing a single pointer, so it can't be faster |
| **free** | ✅ Freeing is just dropping the object and adding it to the free list | ✅ Same |
| **Ergonomics** | ❌ You need a reference to the allocator to access stored elements | ✅ It uses pointer access, so you only need the pointer |
| **Pointer Size** |❌ Although in this case you have a bit more control, you still need at the least two numbers for the handle (index and generation), so the pointer size would be arround 64 and 96 bits | ❌ Pointers are at the least 96 bits long, which can be taxating if you hold too many references to entities |

## Safety Assumptions

Neither `InPlaceAllocator` nor `BoxAllocator` are safe, which doesn't mean they cannot be used safely, it means that their **safety assumptions** cannot be enforced by the Rust compiler. Instead, you have to enforce it within your system at runtime. And to do that, you need a clear understanding of which assumptions we are talking about:

- **The Allocator will live longer than allocated objects:** This means that the allocator should outlive any pointer that it allocated at some point in time. Otherwise, dereferencing a pointer will result in undefined behavior (but most likely a segfault & panic)

    - This doesn't matter with the `InPlaceAllocator` because you need a reference to the allocator, although you might run similar problems if you have several allocators and you need to be careful with your handle usage

- You have to choose a **bad access policy when accessing dead entities**. Do you consider this an unrecoverable error and panic the program? do you consider it a user error and report it with a `Result` type?

- You can have several mutating references to the same entity, so **the system will manage reference creation and access**. This is especially important if your system provides multithreaded access to entities. 

- When using parallelism, a possible **race condition** can happen when someone calls `is_live` (possibly implicitly with pointer dereference) when another thread is `free`ing the entity. So, in this case, you either **wrap the allocator entry type in a mutex-like object**, or **design your system so that this doesn't happen**

## Usage on proto-ecs

To provide an example of how this allocator can be used safely, let's talk about how we do it in [proto-ecs]({% post_url 2024-01-05-proto-ecs-intro-en %}). 

The first thing to note is that the entity allocator is well within the depths of the engine, the **user will never have direct access to the entity allocator**. This is important because it allows the engine to have full control of the allocation, destruction, and access of entities. 

So, let's walk step by step to see how we make this implementation safe:

- **The allocator will live longer than allocated objects** : This one is easy, we create the allocator on engine start-up, and destroy it on engine shut down. Since users have no access to the allocator, there's no way to shut it down before that.

- **Bad Access Policy** : In proto-ecs, we choose to consider accessing a dead entity **a programming error**, and therefore will crash the engine.

- **Reference creation and access** : In proto-ecs, local systems will only have access to their owner entity data groups, so they can't access entity references at all. Global systems provide access to the affected entities as an argument, ensuring that no other thread will access the same entities at the same time, so it's hard to create several mutating references to the same entities. 

- **live-on-free race condition** : We don't allow directly creating/destroying entities with `free` or `new`, we provide functions that will schedule the creation or destruction of an entity for the next frame. This way, we can safely call `free` and `new` when we know there are no `is_live` or `get` calls. It's also worth noting that we have to do this anyway because creating an entity implies much more than just allocating it, several bookkeeping operations have to run, and we do it between frames to keep execution flow consistent. 

## Profiling 

Now we will end this article with simple profiling to have some idea of the performance implications of each implementation. This is not a thorough test by any means, but it helped us to form an idea to decide which implementation to use.

It's worth noting that most operations are asymptotically the same (O(1)), so the performance difference will be most likely due to cache friendliness.

To profile our implementations, we use [criterion](https://github.com/bheisler/criterion.rs), a Rust framework for benchmarking. 

Our two most important performance metrics are **allocation** and **access**, because those are the two most likely to become a bottleneck. 

If you want to check the benchmarking code, you can find it here:

1. [Box Allocator, Allocation Bench](https://github.com/LDiazN/example-allocators/blob/09ad60b21f4a482bd2333c2ae5cdd1500c15fa41/example_allocators/benches/allocator_bench.rs#L69)

2. [Box Allocator, Access Bench](https://github.com/LDiazN/example-allocators/blob/09ad60b21f4a482bd2333c2ae5cdd1500c15fa41/example_allocators/benches/allocator_bench.rs#L85)

3. [In Place Allocator, Allocation Bench](https://github.com/LDiazN/example-allocators/blob/09ad60b21f4a482bd2333c2ae5cdd1500c15fa41/example_allocators/benches/allocator_bench.rs#L111)

4. [In Place Allocator, Access Bench](https://github.com/LDiazN/example-allocators/blob/09ad60b21f4a482bd2333c2ae5cdd1500c15fa41/example_allocators/benches/allocator_bench.rs#L127)

**Allocation benches** will allocate 10k entities, with a for loop, while **access benches** will access an entity and its attributes for 10k entities.

Here are our results:

|                    | Allocation | Access |
| ------------------ | ---------- | --------- |
| `BoxAllocator`     | 8.4544 ms  | 20.378 µs |
| `InPlaceAllocator` | 11.492 ms  | 25.570 µs |

As expected, allocation time is higher with the `InPlaceAllocator` due to resizes of the underlying vector being more expensive because entities are harder to copy than pointers. Access time is also lower with the `BoxAllocator`, which makes sense considering that it's just a pointer dereference. 

We ended up choosing the `BoxAllocator` since it's the most flexible, performant, and ergonomic to use. The drawback is the worst cache efficiency, but this is something that can be solved with a more thoughtful implementation. Safe access to entities is also not a problem, since our system ensures that users can't access entities directly at once. See [this summary of proto-ecs]({% post_url 2024-01-05-proto-ecs-intro-en %})

# 

# Final Thoughts 

In this article, we went from using generational indices for allocating ids to entities, to a full entity allocator. This implementation is not the most comprehensive but it helps as a general outline you can use to create your implementation. We have several implementations with varying degrees of safety and ergonomics that you can choose depending on your needs.

In doing so, we explored many important but non-trivial Rust features, like `MaybeUninit` and `RefCell`. We also implemented unsafe alternatives with important advantages. And although using unsafe is not particularly popular in Rust, is a language feature like any other that we can use if needed as long as we do it right. Remember, using unsafe is just telling the Rust compiler "I will ensure the safety of this code on my own". Always remember to understand the safety conditions to make an unsafe code safe!