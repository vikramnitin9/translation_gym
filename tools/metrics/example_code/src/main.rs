mod fun_method;

use fun_method::{S, T};

fn main() {
    let s = S {};
    unsafe{s.met();}
    s.bla();
}