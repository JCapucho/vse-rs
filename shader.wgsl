[[stage(compute), workgroup_size(1)]]
fn main() -> i32 {
    let a = 1;
    let b = a + 1;
    let c = b + 1;
    return c;
}
