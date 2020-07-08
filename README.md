# Writing Interpreters in Rust: a Guide

![](https://github.com/rust-hosted-langs/book/workflows/code-test/badge.svg)

This is an online book covering the lower level topics involved in writing an
interpreter in Rust including:

* memory management: allocation and garbage collection
* compiling: expressions, functions, closures
* virtual machines: bytecode, instruction dispatch


## Project vision

From CPython to Ruby's YARV, V8 and SpiderMonkey, GHC to the JVM, most language
runtimes are written in C/C++.

We believe that Rust is eminently suitable for implementing languages and can
provide significant productivity improvements over C and C++ while retaining
the performance advantages and low level control of both.

While there are a number of languages implemented in Rust available now, in
varying states of completeness - interpreters, AOT compilers and
JIT-compiled - our vision is singular:

_To create a well documented reference compiler and runtime,
permissively licensed, such that you can fork and morph it into your own
programming language._

That is, a platform for bootstrapping other languages, written in Rust.
To that end, the implementation provided here is not intended to be feature
complete and cannot possibly represent every variation of programming
language or local optimization.

It is a lofty goal, and it certainly won't be the right approach for
everybody. However, we hope it will help shift the landscape in favor of more
memory-safe language implementations.


## Getting involved

See `CONTRIBUTING.md` for licensing and how to get involved.


## The contents

The rendered book can be read [here](https://rust-hosted-langs.github.io/book/)
while the accompanying source code can be browsed in this repository.
