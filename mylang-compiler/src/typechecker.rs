
const NULL: *const () = std::ptr::null();

fn null<T>() -> *const T {
    return std::ptr::null();
}

fn main() {
    //const FUNC: Fn<T>() -> *const T = std::ptr::null;

    let a: *const i32 = NULL;
}

/*
def nullptr: ?&T: Any = Std.Cast.reinterpret(0);

def main() {
    let a: ?&T: Any = nullptr;
    var b: ?&some Any = nullptr;
}
*/

