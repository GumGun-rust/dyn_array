use super::*;


#[test]
fn test(){
    let mut holder:Array<i32> = Array::with_capacity(100).unwrap();
    println!("{holder:#?}");
    let _ = holder.push(9999);
    for elem in 0..1024{
        let _ = holder.push(elem);
    }
    println!("{holder:#?}");
    for elem in 0..1024 {
        let _ = holder.pop();
    }
    println!("{holder:#?}");
    for elem in 0..7000{
        let _ = holder.push(elem);
    }
    println!("{holder:#?}");
    println!("{holder:#?}");
    panic!();
}
