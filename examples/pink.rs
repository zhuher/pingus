fn main() {
    let mut v: Vec<u32> = Vec::from([0xffc0cbff; 480000]);
    for y in 18..24 {
        v[13 + 800 * y] = 0xFF0000FF;
        v[17 + 800 * y] = 0xFF0000FF;
    }
    for x in 8..23 {
        v[x + 800 * 13] = 0xFF0000FF;
    }
    v[7 + 800 * 14] = 0xFF0000FF;
    v[23 + 800 * 14] = 0xFF0000FF;
    v[6 + 800 * 15] = 0xFF0000FF;
    v[24 + 800 * 15] = 0xFF0000FF;
    v[6 + 800 * 16] = 0xFF0000FF;
    v[24 + 800 * 16] = 0xFF0000FF;
    match pingus::create(800, 600, &v, "pink.png") {
        Ok(_) => {
            println!("\x1B[32mThe pink created successfully.\x1B[0m");
        }
        Err(e) => {
            println!("\x1B[33mFailed to create the pink: \x1B[31m{e}\x1B[0m");
        }
    }
}
