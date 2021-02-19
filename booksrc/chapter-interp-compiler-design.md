# Compiler: Design

Drawing from the [VM design](./chapter-interp-vm-design.md), the compiler must
support the following language constructs:

* function definitions
* anonymous functions
* function calls
* lexical scoping
* closures
* local variables
* global variables
* expressions

This is a minimally critical set of features that any further language
constructs can be built on while ensuring that our compiler remains easy to
understand for the purposes of this book.

[Our parser, recall](./chapter-interp-parsing.md), reads in s-expression syntax
and produces a nested `Pair` and `Symbol` based data structure. Adding other
types - integers, strings, arrays etc - is a matter of expanding the parser.
The compiler as described here, being for a dynamically typed language, should
support them without needing to be refactored.


