fn main() {
    let mut v: Vec<u32> = Vec::from([0xFFC0CBFF; 1024]);
    for y in 18..24 {
        v[13 + 32 * y] = 0xFF0000FF;
        v[17 + 32 * y] = 0xFF0000FF;
    }
    for x in 8..23 {
        v[x + 32 * 13] = 0xFF0000FF;
    }
    v[7 + 32 * 14] = 0xFF0000FF;
    v[23 + 32 * 14] = 0xFF0000FF;
    v[6 + 32 * 15] = 0xFF0000FF;
    v[24 + 32 * 15] = 0xFF0000FF;
    v[6 + 32 * 16] = 0xFF0000FF;
    v[24 + 32 * 16] = 0xFF0000FF;
    match pingus::create(32, 32, &v, "pink.png") {
        Ok(_) => {
            println!("\x1B[32mThe pink created successfully.\x1B[0m");
        }
        Err(e) => {
            println!("\x1B[33mFailed to create the pink: \x1B[31m{e}\x1B[0m");
        }
    }
}
