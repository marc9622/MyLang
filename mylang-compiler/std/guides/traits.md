# Traits

## What are Traits

Traits are similar to interfaces or protocols in other languages.
They let you declare a set of functions, types, or values that a type must define.
You could have a trait called `ToStr` that declares a function called `to_str` that converts a type to a `Str`.
Then you could implement this trait for your own types by defining the `to_str` function.
This would let you make generalized code that works with any type that implements the `ToStr` trait.

Here is how such a trait could be defined:

```mylang
trait ToStr {
    for self: *Self {
        virt to_str() -> Str;
    }
}
```

Note that `virt` is used instead of `def` to declare the function.
`virt` is short for *virtual* and is used to declare functions, types, and values that are part of a trait.
These virtual declarations function as a contract that a type must fulfill to implement the trait.
In this example, the `to_str` function does not have a body. This is allowed and common for virtual declarations.
The body is defined when the trait is implemented for a type.
You could also choose to provide a body for virtual functions which would be used as a default implementation.
Default implementations are used when a type does not provide its own implementation.
But since they still use the `virt` keyword, they can still be *overridden* by a type that implements the trait.

