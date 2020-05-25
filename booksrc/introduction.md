# Writing Interpreters in Rust

Welcome!

In this book we will cover some fundamental components of interpreted language
implementation.

At a glance, these are:

* A Sticky (non-moving) Immix allocator and garbage collection implementation
* A simple S-Expression language compiler for a virtual machine

The goal of this book is not to cover a full featured language but rather to
implement a solid foundation on which further features can be built by you!

The focus in many cases is on challenges specific to a Rust implementation.

We hope you find this book to be informative!

Further reading:

* Bob Nystrom - [Crafting Interpreters](http://craftinginterpreters.com/).
  Strongly recommended companion reading to this book.
* Stephen M. Blackburn & Kathryn S. McKinley - 
  [Immix: A Mark-Region Garbage Collector with Space Efficiency, Fast Collection, and Mutator Performance](http://www.cs.utexas.edu/users/speedway/DaCapo/papers/immix-pldi-2008.pdf)
