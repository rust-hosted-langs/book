# Writing Interpreters in Rust: a Guide

## Welcome!

In this book we will walk through the basics of interpreted language 
implementation in Rust with a focus on the challenges that are specifc 
to using Rust.

At a glance, these are:

* A custom allocator for use in an interpreter
* A safe-Rust wrapper over allocation
* A compiler and VM that interact with the above two layers

The goal of this book is not to cover a full featured language but rather to
provide a solid foundation on which you can build further features.

We hope you find this book to be informative!


## Further reading and other projects to study:

All the links below are acknowledged as inspiration or prior art.

### Interpreters

Bob Nystrom's [Crafting Interpreters](http://craftinginterpreters.com/)
is recommended companion reading to this book for beginners to the topic
as there are strong parallels.

### Memory management

* Richard Jones, Anthony Hosking, Elliot Moss - [The Garbage Collection Handbook](http://gchandbook.org/)
* Stephen M. Blackburn & Kathryn S. McKinley - 
  [Immix: A Mark-Region Garbage Collector with Space Efficiency, Fast Collection, and Mutator Performance](http://www.cs.utexas.edu/users/speedway/DaCapo/papers/immix-pldi-2008.pdf)

### Related and interesting Rust projects

* The [Inko](https://gitlab.com/inko-lang/inko) programming language
* The [Gluon](https://github.com/gluon-lang/gluon) programming language
* The [ketos](https://github.com/murarth/ketos) programming language
* jorendorff - [cell-gc](https://github.com/jorendorff/cell-gc)
* kyren - [gc-arena](https://github.com/kyren/gc-arena) and [luster](https://github.com/kyren/luster)

