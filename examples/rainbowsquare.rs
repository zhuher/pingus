fn main() {
    let mut colour: u32 = 0xFF0000FF;
    enum Colourd {
        Redplusgreen,
        Greenminusred,
        Greenplusblue,
        Blueminusgreen,
        Blueplusred,
        Redminusblue,
    }
    let mut cd = Colourd::Redplusgreen;
    let mut data: Vec<Vec<u32>> = Vec::new();
    loop {
        match cd {
            Colourd::Redplusgreen => {
                data.push(Vec::from([colour; 256]));
                colour = colour.saturating_add(0x050000);
                if colour >> 16 & 0xFF == 0xFF {
                    cd = Colourd::Greenminusred;
                }
            }
            Colourd::Greenminusred => {
                data.push(Vec::from([colour; 256]));
                colour = colour.saturating_sub(0x05000000);
                if colour >> 24 & 0xFF == 0x0 {
                    cd = Colourd::Greenplusblue;
                }
            }
            Colourd::Greenplusblue => {
                data.push(Vec::from([colour; 256]));
                colour = colour.saturating_add(0x0500);
                if colour >> 8 & 0xFF == 0xFF {
                    cd = Colourd::Blueminusgreen;
                }
            }
            Colourd::Blueminusgreen => {
                data.push(Vec::from([colour; 256]));
                colour = colour.saturating_sub(0x050000);
                if colour >> 16 & 0xFF == 0x0 {
                    cd = Colourd::Blueplusred;
                }
            }
            Colourd::Blueplusred => {
                data.push(Vec::from([colour; 256]));
                colour = colour.saturating_add(0x05000000);
                if colour >> 24 & 0xFF == 0xFF {
                    cd = Colourd::Redminusblue;
                }
            }
            Colourd::Redminusblue => {
                if colour >> 8 & 0xFF == 0x0 {
                    break;
                }
                data.push(Vec::from([colour; 256]));
                colour = colour.saturating_sub(0x0500);
            }
        }
    }
    match pingus::create_anim(16, 16, &data, "rainbowsquare.png") {
        Ok(_) => {
            println!("\x1B[32mRainbowsquare created successfully.\x1B[0m");
        }
        Err(e) => {
            println!("\x1B[33mFailed to create rainbowsquare: \x1B[31m{e}\x1B[0m");
        }
    }
}
