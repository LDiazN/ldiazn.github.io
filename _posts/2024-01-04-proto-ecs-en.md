---
title: "A Weird Rust Engine: proto-ecs" 
layout: post
author: Luis Diaz
tags: [Rust, Game Engine, proto-ecs]
miniature: https://www.rust-lang.org/logos/rust-logo-512x512-blk.png
description: In this project I'm trying to implement a multithreaded game engine in Rust following Bobby Angelov's object model. This is the indhttps://www.youtube.com/watch?v=jjEsB611kxsex post where I keep track of our progress!
language: en
repo: 
es_version: false
---

**TL;DR:** This is an index for a series of articles where I document the progress in some key areas of a project I'm working on with a friend to implement a multithreaded game engine based on an interesting object model. 

---

A few months ago a good friend of mine, [Christian Oliveros](https://github.com/maniatic0), shared with me an amazing video from Bobby Angelov where he explains an interesting architecture for a game engine's object model:

<iframe width="400px" height="280px" style="display: block;
  margin-left: auto;
  margin-right: auto;
  "
src="https://www.youtube.com/embed/jjEsB611kxs">
</iframe> 

In this video Bobby starts with a simple crash course about game engine models, their characteristics and problems, emphasizing the problem of debugging and parallelizing gameplay code for a traditional engine. 

Then he proceeds with the most interesting part for us, an architecture for an object model where he attacks most of these problems. It's interesting because the result is a design where parallelism is trivial and errors are harder occur in the first place. 

Chris is an experienced engine programmer and he wanted to give it a try to this architecture, and since he knows that I want to learn more about engine development and that I like Rust, he invited me to work with him on his own implementation which is made in Rust. 

We have had some cool advances in our project so far and since it's quite a long journey, I wanted to start documenting some of our most interesting solutions to problems that we had, and that's a big part of the purpose of this blog. 



## Article Index

This is a section that I will update from time to time whenever I write a new article about this project. Don't expect this to be a step by step tutorial from zero-to-hero about the entire project, since even I don't know where this is going. I just want to write about interesting bits of project that we have had to solve, since we had more and weirder than I expected. I hope you learn as much as I learnt working on this project :)



