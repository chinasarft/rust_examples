
use std::vec::Vec;
fn main() {
    for x in 0 .. 5 {
        println!("--{}", x)
    }
    let xx = (0..5_i32).zip(0..7_i32).collect::<Vec<(i32,i32)>>();
    for x in xx {
      println!("--{:?}", x)
    }
}