# Compiler: Implementation

Before we get into eval and apply we need to consider how we will support lexical
scoping with data structures for keeping track of:

- function local variables
- nested scopes within functions
- references to nonlocal variables in outer function definitions


