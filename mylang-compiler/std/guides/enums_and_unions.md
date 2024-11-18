# Enums and Unions

## What is an Enum?

An enum (or *enumeration*) is a type that represents a set of named values.

Suppose we have some known number of colors that we want to represent in our program.
Additionally, each color variant should have a string value with its name.
For this we could simply use this the `Str` type and define some constants.

```mylang
def red    = "Red";
def yellow = "Yellow";
def green  = "Green";
def blue   = "Blue";
```

We probably want to be able to use these colors in our program.

```mylang
def paint_background(color: ColorName) -> Void {
    // ...
}
```

Though, this would mean that all the places where we expect a color, we could get any string.
This is not ideal, as we would like to restrict the possible values to only the colors we want.
This is what enums are for. We could define an enum like this:

```mylang
def red    = "Red";
def yellow = "Yellow";
def green  = "Green";
def blue   = "Blue";

enum ColorName {
    red;
    yellow;
    green;
    blue;
}
```

This would allow us to use the `ColorName` type in our program.
Here is an example of how we could use it:

```mylang
def main() -> Void {
    // You can get a enum variant by using its name.
    let favorite_color: ColorName = ColorName.red;
    
    #assert(is_color_red(favorite_color));
}

def is_color_red(color: ColorName) -> Bool {

    // The compiler can infer that the enum values of `ColorName` have the type `Str`.
    // We can therefore get the string value of a variant by using the `value` method.
    return color.value() == "Red";
}
```

## Checking if a value is an enum variant

The `is` operator can be used to check if a value is a certain enum variant.

```mylang
def main() -> Void {
    if (favorite_color is red)
        println("My favorite color is red!");
    else
        println("My favorite color is not red :(");
}

def get_rgb(color: ColorName) -> struct {r: U8, g: U8, b: U8} {
    return color
        if   (color is red)    .{255, 0,   0  }
        elif (color is yellow) .{255, 255, 0  }
        elif (color is green)  .{0,   255, 0  }
        elif (color is blue)   .{0,   0,   255}
        else #panic("Unknown color!");
}
```

Writing out all the if statements can get tedious.
Instead, we could use the block form of the `is` operator.

```mylang
def get_rgb(color: ColorName) -> struct {r: U8, g: U8, b: U8} {
    return color is {
        (red)    .{255, 0,   0  };
        (yellow) .{255, 255, 0  };
        (green)  .{0,   255, 0  };
        (blue)   .{0,   0,   255};
    };
}

def is_color_valid(color: ColorName) -> Bool {
    return color is {
        (red, yellow, green, blue) true;
        else false;
    };
}
```

## Enum tag

If we look at the function `is_color_valid` above, it might not be obvious why we would ever need such a function.
Since an enum value can only be one of its variants, this function should always return `true`, right?
Well, not quite. First we need to understand how enums are represented in memory.
The way enums are represented in memory is by using a *tag value*.
By default, this tag is a number of type `UInt8`. The tag used to determine which variant an enum variant is.
This is what allows enums to be efficiently represented in memory and checked for equality.

If we wanted to use a different type as the tag, we could do so in the enum declaration with the `tag` keyword.
The `tag` keyword can also be used to specify the tag values of the enum variants.

```mylang
enum ColorName tag U8 {
    red    tag 0;
    yellow tag 1;
    green  tag 2;
    blue   tag 3;
}
```

We can get the tag of an enum variable by using the `tag` method.

```mylang
def main() -> Void {
    let color = ColorName.green;
    #assert(color.tag() == 2);
}
```

We can also turn a tag value into an enum variant by reinterpret casting it.

```mylang
import Std.Cast

def favorite_color: ColorName = Cast.reinterpret(0::U8);

def invalid_color: ColorName = Cast.reinterpret(123::U8);
```

But as you can see, this is unsafe, as it doesn't stop us from creating invalid enum variants.
You should therefore always be cautious when creating an enum variant directly from a tag value.
This is why the `is_color_valid` function from before can be useful.
It allows us to check if a tag value is a valid enum variant.
You usually shouldn't have to worry about receiving invalid enum variants.
It should be the caller's responsibility to ensure that the tag value is valid.

In cases where the compiler requires that a value is exhaustively matched, such as with the block form `is`,
the compiler will automatically insert panics in cases where the tag value is invalid.
Here is an example:

```mylang
def exhaustive_is(color: ColorName) -> Str {
    return color is {
        (red)    "I like this color."
        (yellow) "This color is fine."
        (green)  "This color is okay."
        (blue)   "I don't like this color."
        // else #panic("Unknown color!");
    };
}
```

Here the compiler would insert an else branch that panics if the tag value didn't match any of the variants.

## Various ways of declaration enums

In the examples above, we defined the enum variants outside of the enum declaration.
You can also define the enum variants inside the enum declaration.

```mylang
enum ColorName {
    def red    = "Red";
    def yellow = "Yellow";
    def green  = "Green";
    def blue   = "Blue";
}
```

And if you also want to specify the tag type and values, you can do so like this:

```mylang
enum ColorName tag U8 {
    def red    tag 0 = "Red";
    def yellow tag 1 = "Yellow";
    def green  tag 2 = "Green";
    def blue   tag 3 = "Blue";
}
```

As it is written above, the compiler will infer that the value type of the enum variants is `Str`.
If you want to specify the value type, you can do so like this:

```mylang
enum ColorName of Str tag U8 {
    def red    tag 0 = "Red";
    def yellow tag 1 = "Yellow";
    def green  tag 2 = "Green";
    def blue   tag 3 = "Blue";
}
```

## Terminology

* **Enum (type)**: A type that represents a set of named values.
`ColorName` below is an enum.
* **Enum variant**: One of the named values of an enum.
`red`, `yellow`, `green` and `blue` below are enum variants.
* **Enum value**: The value associated with an enum variant.
`"Red"`, `"Yellow"`, `"Green"` and `"Blue"` below are enum values.
* **Enum tag (value)**: The tag of an enum variant used to determine which variant it is.
`0`, `1`, `2` and `3` below are enum tags.
* **Enum tag type**: The type of the tag of an enum.
`U8` below is the enum tag type.
* **Enum variable**: A variable that holds an enum variant.
`favorite_color` below is an enum variable.

```mylang
enum ColorName tag U8 {
    def red    tag 0 = "Red";
    def yellow tag 1 = "Yellow";
    def green  tag 2 = "Green";
    def blue   tag 3 = "Blue";
}

def favorite_color = ColorName.red;
```

## What is a Union?

Whereas an enum represents a set of named values, a union is a type that represents a set of other named types.

Suppose we want to add support for custom colors to our program.
We could do this by defining a struct that holds the red, green and blue values of the color.

```mylang
struct ColorRgb {
    r: U8;
    g: U8;
    b: U8;
}
```

But how would we represent a value than can be either a `ColorName` or a `ColorRgb`?
We could use a union for this.

```mylang
union Color {
    ColorName;
    ColorRgb;
}
```

This would allow us to use the `Color` type in our program.

```mylang
def paint_background(color: Color) -> Void {
    // ...
}

def main() -> Void {
    let favorite_color: Color = ColorName.red;

    paint_background(favorite_color);

    let custom_color: Color = ColorRgb.{255, 0, 0};

    paint_background(custom_color);
}
```

## Checking if a value is a union variant

The `is` operator can also be used to check if a value is a certain union variant.

```mylang
def describe_color(color: Color, alloc: &Allocator) -> Str throws {
    return
        if (color is ColorName name)
            "Color is \(name)".format(&alloc).#throw;
        else
            "Color is a custom color".format(&alloc).#throw;
}
```

