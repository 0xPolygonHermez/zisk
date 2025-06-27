mod lib_c;

pub use lib_c::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_test() {
        println!("lib c simple test!");

        {
            let dbl: u64 = 0;
            let x1: [u64; 4] = [3729, 9457, 4565, 3454];
            let y1: [u64; 4] = [3452323, 756765, 6543, 23575];
            let x2: [u64; 4] = [645623, 76867, 74543, 67665];
            let y2: [u64; 4] = [213456, 654, 2343456, 76543];
            let mut x3: [u64; 4] = [0, 0, 0, 0];
            let mut y3: [u64; 4] = [0, 0, 0, 0];
            let result = add_point_ec_c(dbl, &x1, &y1, &x2, &y2, &mut x3, &mut y3);
            if result == 0 {
                println!("lib c simple test successfully called add_point_ec_c() x1={:x}:{:x}:{:x}:{:x} y1={:x}:{:x}:{:x}:{:x} x2={:x}:{:x}:{:x}:{:x} y2={:x}:{:x}:{:x}:{:x} x3={:x}:{:x}:{:x}:{:x} y3={:x}:{:x}:{:x}:{:x}",
                x1[3], x1[2], x1[1], x1[0],
                y1[3], y1[2], y1[1], y1[0],
                x2[3], x2[2], x2[1], x2[0],
                y2[3], y2[2], y2[1], y2[0],
                x3[3], x3[2], x3[1], x3[0],
                y3[3], y3[2], y3[1], y3[0]
            );
            } else {
                println!("lib c simple test failed calling add_point_ec_c() result={result}");
            }
        }

        {
            let dbl: u64 = 0;
            let p1: [u64; 8] = [3729, 9457, 4565, 3454, 3452323, 756765, 6543, 23575];
            let p2: [u64; 8] = [645623, 76867, 74543, 67665, 213456, 654, 2343456, 76543];
            let mut p3: [u64; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
            let result = add_point_ec_p_c(dbl, &p1, &p2, &mut p3);
            if result == 0 {
                println!("lib c simple test successfully called add_point_ec_p_c() x1={:x}:{:x}:{:x}:{:x} y1={:x}:{:x}:{:x}:{:x} x2={:x}:{:x}:{:x}:{:x} y2={:x}:{:x}:{:x}:{:x} x3={:x}:{:x}:{:x}:{:x} y3={:x}:{:x}:{:x}:{:x}",
                p1[3], p1[2], p1[1], p1[0],
                p1[7], p1[6], p1[5], p1[4],
                p2[3], p2[2], p2[1], p2[0],
                p2[7], p2[6], p2[5], p2[4],
                p3[3], p3[2], p3[1], p3[0],
                p3[7], p3[6], p3[5], p3[4]
            );
            } else {
                println!("lib c simple test failed calling add_point_ec_p_c() result={result}");
            }
        }
    }
}
