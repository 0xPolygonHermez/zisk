mod lib_c;

pub use lib_c::*;

// pub fn add(left: u64, right: u64) -> u64 {
//     left + right
// }

#[cfg(test)]
mod tests {
    use super::*;

    //#[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
    #[test]
    fn simple_test() {
        println!("lib c simple test!");
        let dbl: u64 = 0;
        let x1: [u64; 2] = [3729, 9457];
        let y1: [u64; 2] = [3452323, 756765];
        let x2: [u64; 2] = [645623, 76867];
        let y2: [u64; 2] = [213456, 654];
        let mut x3: [u64; 2] = [0, 0];
        let mut y3: [u64; 2] = [0, 0];
        let result = add_point_ec_c(dbl, &x1, &y1, &x2, &y2, &mut x3, &mut y3);
        if result == 0 {
            println!("lib c simple test successfully called AddPointEc() x1={:x}:{:x} y1={:x}:{:x} x2={:x}:{:x} y2={:x}:{:x} x3={:x}:{:x} y3={:x}:{:x}",
            x1[0], x1[1],
            y1[0], y1[1],
            x2[0], x2[1],
            y2[0], y2[1],
            x3[0], x3[1],
            y3[0], y3[1]
        );
        } else {
            println!("lib c simple test failed calling AddPointEc() result={}", result);
        }
    }
}
