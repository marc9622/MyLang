# Reference types

## What are the reference types?

References and pointers are used to store a memory address that may or may not point to some data.
There are multiple types available to represent such address. Theese are called *reference types*.
The core library comes with multiple reference types, each with their own semantics and use cases.
Here is a list of the reference types covered in this document:
- `&T`, called a *reference*, and represents a mutable reference to some value.
- `*T`, called a *view*, and represents a readonly reference to some value.
- `^T`, called a *pointer*, and represents a pointer to some memory, which may or may not be valid memory.

But there are also more such as the `Own(T)` and `Rc(T)` types.
The three listed above are called the *primitive reference types* and are given special symbols in the language.
The types, `Ref(T)`, `View(T)` and `Ptr(T)`, respectively, are aliases and refer to the *exact* same types.

### The rationale behind the symbol choices

- `^` was chosen for pointers because it also used for pointers in some other programming languages as well.
It is also a slightly uncommon symbol and is not always as easy to type on all keyboard layouts.
This fits pointers because they should be used sparingly and with caution.
- `&` and `*` were chosen because they are symbols used for pointer-/reference-like types in a lot of languages.
- `&` was chosen for references because a lot of other languages uses it for the same purpose.
- `*` was chosen for view because it is typically smaller than an `&` visually.
This fits views because they are more restrictive and safer than references and are used more often.

## Referencing and dereferencing

The primitive reference types work a little different compared to many other languages.
Here are some points to keep in mind:
- The prefix operators `&`, `*`, and `^` act more like conversion operators than address-of operators.
- References are automatically demoted into views on use, and need to be explicitly promoted back into references.
- Views are created implicitly, but can still be created explicitly.
- Pointers and references require explicit creation, and can't be created implicitly.

### Getting a reference

References are created by using the `&` operator, and can be created from any mutable value type.

```mylang
var value: Int = 3;     // value must be mutable to get a reference to it
let ref: &Int = &value;
```

Using a variable that is a reference where a reference is expected also requires the `&` operator.
That is because references are automatically demoted into views on use.

```mylang
def use_ref(_: &Int) -> return;
def use_view(_: *Int) -> return;

for _: &Int def use_ref_method() -> return;
for _: *Int def use_view_method() -> return;

def function(ref: &Int) -> {
    // ref_copy is of type *Int due to automatic demotion
    var ref_copy = ref;

    use_ref(&ref); // Explicitly used as a reference
    use_view(ref); // Automatically demoted to a view
    
    ref.use_view_method();
    &ref.use_ref_method();
}
```

It ensures that it is always very easy to see whether a function is mutating a value or not.
It means that reusing a reference and getting a reference to a value has identical syntax.
Changing a variable or parameter from a value to a reference (or vice versa) is therefore a very simple change.

```mylang
def function(value: Int, ref: &Int) -> {
    use_ref(&ref);
    use_ref(&value);
    use_view(ref);
    use_view(value);
    
    &ref.use_ref_method();
    &value.use_ref_method();
    ref.use_view_method();
    value.use_view_method();
}
```

### Getting a view

Views are created implicitly, but can also be created explicitly with the `*` operator.

```mylang
def use_view(_: *Int) -> return;

def function(value: Int, ref: &Int, view: *View) -> {
    use_view(value);
    use_view(*value);
    use_view(ref);
    use_view(*ref);
    use_view(view);
    use_view(*view);
}
```

### Getting a pointer

Pointers are created explicitly with the `^` operator, and can be created from any value type or a reference.

```mylang
def use_ptr(_: ^Int) -> return;

def function(int: Int, ref: &Int, view: *Int) -> {
    use_ptr(^int);
    use_ptr(^ref);
    // use_ptr(^view); // Error: Can't create a pointer from a view
}
```

Pointers can be turned into references, which are mutable. Thats why you can't turn a view into a pointer.

### Dereferencing a reference type

To get the value pointed to by any reference type, the `.^` operator is used.
If the value is not implicitly copyable, then the `copy` keyword needs to be used to explicitly copy the value.
The `copy` keyword ensures that values aren't copied unknowingly.
Copying values without thought can lead to performance issues or might violate some invariants.

```mylang
def function(int_ref: &Int, seq_ref: &[1]Int) -> {
    let _: Int = int_ref.^;         // Ok: Int is implicitly copyable
    //let _: [1]Int = seq_ref.^;    // Error: Sequences are not implicitly copyable
    let _: [1]Int = copy seq_ref.^; // Ok: Explicitly copied
}
```

The `.^` can also be chained to dereference multiple references.

```mylang
def function(int_ref: &&Int) -> {
    let a: Int = int_ref;
}
```

The `.^` operator can be implemented by any type using the `Deref` trait.
Other reference types such as `Own(T)` and `Rc(T)` also implement this trait.
A `Deref` implementation is usually the thing that all reference types have in common.

### Accessing members of a reference type

Most reference types implement the `Member` trait, which field access using the `.` operator.

```mylang
struct Person {
    name: Str;
    age: Int;
}

def function(person: &Person) -> {
    let _: *Str = person.name; // Views implement the `Member` trait,
                               // so we can still access `name` as a view.
    let _: &Str = &person.name;

    println(person.name);
    println(person.age);
}
```

`Member` is a sub trait of the `Deref` trait, meaning that member access is not possible on all reference types.
One such type is the pointer type. This is because pointers might not point to valid memory.
This forces you to explicitly dereference the pointer or turn it into another reference type.  
*See `Pointer to other reference types` at the bottom.*

## Converting reference types

As shown, a reference can easily be turned into a view, but a view can't be turned into a reference.
Views cannot be turned into other primitive reference types to ensure that they are always readonly.
But what if you want get a reference to a view?

### Referencing a reference type

You can get a reference type to another reference type by combining the prefix operators.
Here are some examples of converting between reference types:  

```mylang
def function(var ref: &Int, var view: *Int, var ptr: ^Int) -> {
    let _:  &Int =  &ref;
    let _: &&Int = &&ref;
    let _: *&Int = *&ref;
    let _: ^&Int = ^&ref;

    let _:  *Int =  *view; // or just `view`
    let _: **Int = **view;
    let _: &*Int = &*view;
    let _: ^*Int = ^*view;

    let _:  ^Int =  ^ptr;
    let _: ^^Int = ^^ptr;
    let _: *^Int = *^ptr;
    let _: &^Int = &^ptr;
}
```

*Notice how the prefix operators tend to match the resulting reference type?*

### Pointer to other reference types

Because pointers might not point to valid memory, they don't have any implicit conversions.
Luckily, there are some functions that can be used to convert pointers to other reference types.

```mylang
def function(ptr: ^Int) -> {
    let _: *Int = ptr.as_view();
    let _: &Int = ptr.as_ref();
}
```

