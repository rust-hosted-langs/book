# Tagged pointers and symbols

In the previous chapter, we introduced a pointer type `ScopedPtr<T>`. This
pointer type has compile time knowledge of the type it is pointing at.

In our interpreter we won't always have that. As a dynamic language
interpreter, our compiler won't do type checking. We'll depend on runtime
type identification in our virtual machine.

In Python, for example, the following code does not have compile time
protection against passing in strings:

```python
def multiply(a, b):
    return a * b

multiply("bob", "alice")
```

This script will result in a runtime error and not a compile time error.
Our lannguage will behave similarly.

For this to work, we need an alternative to `ScopedPtr<T>` that does not
care about compile time types _but_ from which the type can be resolved
at runtime.

We'll spend some time now inventing some new pointer types to support this.
