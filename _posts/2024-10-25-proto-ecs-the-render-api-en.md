---
title: "[proto-ecs] The Render API"
layout: post
author: Luis Diaz
tags: [Rust, Game Engine, proto-ecs, Computer Graphics]
miniature: /assets/images/proto-ecs/proto-ecs-first-render.gif
description: TODO
language: en
repo: 
es_version: false
---

![The first 3D object we loaded and rendered. The version in this gif is using phong shading for debugging](/assets/images/proto-ecs/proto-ecs-first-render.gif)

As any other fan of game engine programming, one of the milestones that I was most excited about was the render, and after many lines of code, we finally have (sort of) a render to display 3D models in screen!

In this article I will share how we structured and implemented the Render API and how the render thread fits into the architecture of  proto-ecs. 

The low level API design is heavily influenced by [this talk from Sebastian Aaltonen](https://www.youtube.com/watch?v=m3bW8d4Brec) and the general render architecture is inspired  by [this article](https://blog.mecheye.net/2023/09/how-to-write-a-renderer-for-modern-apis/).

## API Structure

For the render API we wanted the following things: 

1. **No leaking** of platform-specific details into the render code. 
2. **Easy implementation** of new render APIs, specially for maintainment.

So, following Sebastian's ideas, we decided to use a thin layer over the graphics APIs, resulting in the following structure:  

- **Low Level**: This level is **just a thin abstraction** over the Render API (OpenGL, Vulkan, Metal). It tries to be as non-abstract as possible, like using the actual platform-specific API without being platform-specific. Its sole purpose is to prevent other parts of the engine from using platform specific code. To add support for a new API, you _just_ have to implement the `RenderAPIBackend` trait. It works with Index/Vertex Buffers and Shaders.

- **Mid Level**: Includes models, materials, lights, camera, built upon the Low Level API. Implements the basic building blocks to show something on screen. It doesn't care about which render API is being used, it does not make any assumptions on that. 

- **High Level**: Includes high level things usually explicitly controlled by the user and built upon the Mid Level API, like UI Rendering, animations, and particles. This level is not yet implemented, but that's the idea.

![Render API Structure](/assets/images/proto-ecs/render-api/RenderAPIStructure.png)

For now we only support OpenGL, and we are currently using [Phong Shading](https://en.wikipedia.org/wiki/Phong_shading) with fixed parameters to test this structure, and the next step will be implementing PBR rendering.

## Entity system

Integrating the rendering system to the entity system was actually pretty easy using datagroups, local systems and global systems. We defined a new global system: `MeshRenderer`, and a new datagroup: `MeshRenderer`.

> More about **Datagroups, Global Systems, and Local Systems** [here]({% post_url 2024-01-05-proto-ecs-intro-en %}) 

Users specify a scene by adding a `MeshRenderer` datagroup to the entities they want to display. The `MeshRenderer` datagroup also 
requires the object to have a `Transform` datagroup, in order to specify its position, scale and orientation. 

The `MeshRenderer` allows entities to specify a list of models and materials to use for those models. Then the `RenderGS` global system will run at the end of each frame to collect all information in scene needed to render a frame and it will send it to the **Render Thread**. This information includes lights, cameras, transforms, models, and materials. We use **double buffering**, so the render thread will always draw the previous frame, while the simulation is a frame ahead. 

By just specifying models, lights, materials, and a camera, the user is able to describe a scene. However, how does this information flow through the engine? Enter the render thread:

## Render Thread

While the main thread is running the simulation and updating the entity system, the render thread is drawing the previous frame in parallel. The main and render threads communicate by **sharing memory**. There is `RenderThreadSharedStorage` struct that holds all the information shared with the main thread, separated from the memory internally used by the render thread. For now it looks like this: 

```rust
pub struct RenderSharedStorage {
    last_frame_finished: AtomicBool,
    running: AtomicBool,
    started: AtomicBool,

    /// Description of the next frame.
    frame_desc: RwLock<FrameDesc>,

    /// Store shaders by name, for easier retrieval
    name_to_shaders: RwLock<HashMap<String, ShaderHandle>>,
}
```

Rather than locking the entire struct behind a mutex or `RwLock`, we define the fields such that each one can be accessed independently, hence the atomics and `RwLock`s. The shared storage contains some metadata about the current state of the render thread and, most importantly, the `frame_sec` variable, which holds the information required to render a frame. This is the variable that gets updated by the main thread and consumed by the render thread. 

Both the main thread and the render thread have a **sync point** where they operate this data. At the end of each frame, the main thread will lock the `frame_desc` variable and update the frame description with the information collected from the entity system. Then, when the render thread is finished with the previous frame, it will also lock the `frame_desc` variable and move its content to its interntal memory, to free the lock as soon as posible. 

![Thread communication by shared memory](/assets/images/proto-ecs/render-api/RenderAPI-ThreadCommunication.png)

What happens when the main thread already finished a new frame, but the render thread is not finished with the previous one? There are multiple approaches to this problem: 

1. **Skip the frame**: Just replace the undrawn frame with the next frame. This approach is very easy to implement and kind of makes sense, because it's already too late to draw the frame in buffer, but it can cause issues when the render thread slows down, because if it happens too many times in a row the next drawn frame will have a state wildly different to the previous one, causing a bad experience for the user. 
2. **Wait for the render thread to finish**: Skip the next simulation step until the render frame is finished.  This approach can cause stutter, but at the least the state won't change significanlly between frames.

There are probably more and more sophisticated approaches to this problem, but for now these are the most direct ones, and we are currently used the first one since it's the easiest to implement.

## Resource lifetime and allocation

A key part of this design is how resources are managed. Usually, the creation of a 





