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

As any other fan of game engine programming, one of the milestones that I was most excited about was the render, and after many months of work, we finally have (sort of) a render to display 3D models in screen!

In this article I will share how we structured and implemented the Render API and how the Render Thread fits into the architecture of  proto-ecs.

We splitted the API into three levels:

- **Low Level**: This level is just a thin abstraction over the Render API (OpenGL, Vulkan, Metal). 

