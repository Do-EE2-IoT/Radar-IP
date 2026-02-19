struct Point {
    x: i32,
    y: i32,
    z: i32,
}

fn add_points(p: &mut Point, q: &Point) {
    p.x += q.x;
    p.y += q.y;
    p.z += q.z;
}

// review:
/**
 *  let x = &y;
 *  let t = &mut z;
 *  Khai báo biến không bao giờ có &, chỉ có let, let mut
 *  Phía trên hàm 9 tham số truyền vào, được dùng & or &mut bên phải
 *  tham số bên phải = cũng được dùng & và &mut
 */
fn main() {
    let mut hi: String = String::from("Hello");
    hi.push_str("Happy new year !!!");
    println!("Hello, world! {}", hi);
    let point = Point { x: 1, y: 2, z: 3 };
    let mut point_2: Point = Point { x: 4, y: 5, z: 6 };
    point_2.x = 7;
    println!("Point 1: ({}, {}, {})", point.x, point.y, point.z);
    println!("Point 2: ({}, {}, {})", point_2.x, point_2.y, point_2.z);

    let numbers = vec![10, 20, 30];


    let mut i = 0;
    for num in numbers.iter() {
        println!("Number: {}", num); // save 
    }
    while i <= numbers.len() {
        println!("Number: {}", numbers[i]);
        i += 1; // dead code, sẽ bị lỗi out of bounds khi i = 3
    }
}
