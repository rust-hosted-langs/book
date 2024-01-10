# Writing Interpreters in Rust: a Guide

## Welcome!

In this book we will walk through the basics of interpreted language
implementation in Rust with a focus on the challenges that are specific
to using Rust.

At a glance, these are:

* A custom allocator for use in an interpreter
* A safe-Rust wrapper over allocation
* A compiler and VM that interact with the above two layers

The goal of this book is not to cover a full featured language but rather to
provide a solid foundation on which you can build further features. Along
the way we'll implement as much as possible in terms of our own memory
management abstractions rather than using Rust std collections.

### Level of difficulty

Bob Nystrom's [Crafting Interpreters](http://craftinginterpreters.com/)
is recommended _introductory_ reading to this book for beginners to the topic.
Bob has produced a high quality, accessible work and while there is
considerable overlap, in some ways this book builds on Bob's work with some
additional complexity, optimizations and discussions of Rust's safe vs unsafe.

**We hope you find this book to be informative!**


## Further reading and other projects to study:

All the links below are acknowledged as inspiration or prior art.

### Interpreters

* Bob Nystrom's [Crafting Interpreters](http://craftinginterpreters.com/)
* [The Inko programming language](https://inko-lang.org/)
* kyren - [luster](https://github.com/kyren/luster) and [gc-arena](https://github.com/kyren/gc-arena)

### Memory management

* Richard Jones, Anthony Hosking, Elliot Moss - [The Garbage Collection Handbook](http://gchandbook.org/)
* Stephen M. Blackburn & Kathryn S. McKinley -
  [Immix: A Mark-Region Garbage Collector with Space Efficiency, Fast Collection, and Mutator Performance](http://users.cecs.anu.edu.au/~steveb/pubs/papers/immix-pldi-2008.pdf)
* Felix S Klock II - [GC and Rust Part 0: Garbage Collection Background](http://blog.pnkfx.org/blog/2015/10/27/gc-and-rust-part-0-how-does-gc-work/)
* Felix S Klock II - [GC and Rust Part 1: Specifying the Problem](http://blog.pnkfx.org/blog/2015/11/10/gc-and-rust-part-1-specing-the-problem/)
* Felix S Klock II - [GC and Rust Part 2: The Roots of the Problem](http://blog.pnkfx.org/blog/2016/01/01/gc-and-rust-part-2-roots-of-the-problem/)
