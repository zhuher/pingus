// use crate::example::*;
#![allow(unused)]
mod png {
    pub(crate) enum Chunk {
        Sign,
        IHDR(u32, u32, u8, u8, u8, u8, u8),
        IDAT(u32, u32, Vec<u8>),
        IEND,
    }
    impl Chunk {
        fn format(chunk: &[u8]) -> Vec<u8> {
            Vec::from_iter(
                [
                    &(chunk[4..].len() as u32).to_be_bytes(),
                    &chunk[..4],
                    &chunk[4..],
                    &(crate::crc32::Crc32::from(&[&chunk[..4], &chunk[4..]].concat()).fin())
                        .to_be_bytes(),
                ]
                .concat(),
            )
        }
        pub(crate) fn form_chunk(self) -> Vec<u8> {
            match self {
                Chunk::Sign => Vec::from(&b"\x89PNG\r\n\x1a\n"[..]),
                Chunk::IHDR(
                    width,
                    height,
                    bit_depth,
                    col_type,
                    comp_method,
                    filt_method,
                    interl_method,
                ) => match (bit_depth, col_type) {
                    (8, 6) => Chunk::format(&Vec::from_iter(
                        [
                            &b"IHDR"[..],
                            &width.to_be_bytes(),
                            &height.to_be_bytes(),
                            &[bit_depth, col_type, comp_method, filt_method, interl_method],
                        ]
                        .concat(),
                    )),
                    (_, _) => todo!(),
                },
                Chunk::IDAT(width, height, data) => {
                    let width_byte_4 = width << 2;
                    let final_len = (width_byte_4 + 1) * height;
                    let mut chunk_data: Vec<u8> = Vec::with_capacity(final_len as usize);
                    let mut window: u32 = (height - 1) * width_byte_4;
                    loop {
                        chunk_data.push(0);
                        chunk_data.extend(&data[window as usize..(window + width_byte_4) as usize]);
                        if window == 0 {
                            break;
                        }
                        window -= width_byte_4;
                    }
                    assert_eq!(final_len, chunk_data.len() as u32);
                    Chunk::format(&Vec::from_iter(
                        [
                            &b"IDAT"[..],
                            &/*deflate_bytes_zlib*/super::nondeflate::compress(&chunk_data)[..],
                        ]
                        .concat(),
                    ))
                }
                Chunk::IEND => Chunk::format(&Vec::from(&b"IEND"[..])),
            }
        }
    }
}
mod crc32 {
    pub struct Crc32 {
        table: [u32; 256],
        value: u32,
    }
    const CRC32_POLYNOMIAL: u32 = 0xEDB88320;
    impl Crc32 {
        pub fn new() -> Crc32 {
            Crc32 {
                table: core::array::from_fn::<u32, 256, _>(|i| {
                    (0..8).fold(i as u32, |acc, _| {
                        if acc & 1 != 0 {
                            CRC32_POLYNOMIAL ^ (acc >> 1)
                        } else {
                            acc >> 1
                        }
                    })
                }),
                value: 0xFFFFFFFF,
            }
        }
        ///
        /// This just calls new and update on bytes
        ///
        pub fn from(bytes: &[u8]) -> Crc32 {
            let mut crc: Crc32 = Crc32::new();
            crc.update(bytes);
            crc
        }

        pub fn update(&mut self, bytes: &[u8]) {
            self.value = bytes.iter().fold(self.value, |acc, &i| {
                self.table[((acc ^ u32::from(i)) & 0xFF) as usize] ^ (acc >> 8)
            });
        }

        pub fn fin(&mut self) -> u32 {
            self.value ^ 0xFFFFFFFF
        }
    }
}
// mod huffman {}
// mod lz77 {}
mod adler {
    pub struct Adler32 {
        a: u32,
        b: u32,
    }
    const ADLER_MOD: u32 = 0xFFF1;
    impl Adler32 {
        pub(crate) fn new() -> Adler32 {
            Adler32 { a: 1, b: 0 }
        }

        pub(crate) fn update(&mut self, bytes: &[u8]) {
            bytes.iter().for_each(|&i| {
                self.a = (self.a.wrapping_add(u32::from(i))) % ADLER_MOD;
                self.b = (self.a.wrapping_add(self.b)) % ADLER_MOD
            })
        }
        ///
        /// This just calls new and update on bytes
        ///
        /*fn _from(bytes: &[u8]) -> Adler32 {
            let mut adler = Adler32::new();
            adler.update(bytes);
            adler
        }*/

        pub(crate) fn fin(&self) -> u32 {
            (self.b << 16) | self.a
        }
    }
}
mod nondeflate {
    use super::adler::Adler32;

    pub fn compress(data: &[u8]) -> Vec<u8> {
        const CHUNK_SIZE: usize = 65530;

        let final_len =
            // header
            2 +
            // every chunk adds 5 bytes [1:type, 4:size].
            (5 * {
                let n = data.len() / CHUNK_SIZE;
                // include an extra chunk when we don't fit exactly into CHUNK_SIZE
                n + {if data.len() == n * CHUNK_SIZE && !data.is_empty() { 0 } else { 1 }}
            }) +
            // data
            data.len() +
            // crc
            4
        ;

        let mut raw_data = Vec::with_capacity(final_len);
        // header
        raw_data.extend(&[120, 1]);
        let mut pos = 0;
        let mut adl = Adler32::new();
        for chunk in data.chunks(CHUNK_SIZE) {
            let chunk_len = chunk.len();
            pos += chunk_len;
            let is_last = pos == data.len();
            raw_data.extend(&[
                // type
                if is_last { 1 } else { 0 },
                // size
                (chunk_len & 0xff) as u8,
                ((chunk_len >> 8) & 0xff) as u8,
                (0xff - (chunk_len & 0xff)) as u8,
                (0xff - ((chunk_len >> 8) & 0xff)) as u8,
            ]);

            raw_data.extend(chunk);
            adl.update(chunk);
        }

        raw_data.extend(&adl.fin().to_be_bytes());

        assert_eq!(final_len, raw_data.len());
        raw_data
    }
}
mod canvas {
    #[derive(Debug)]
    pub(crate) struct Canvas {
        pub(crate) data: Vec<u32>,
        pub(crate) w: u32,
        pub(crate) h: u32,
    }
    #[derive(Debug)]
    pub struct Point {
        pub(crate) x: isize,
        pub(crate) y: isize,
    }
    impl Canvas {
        pub(crate) fn new(colour: u32, w: u32, h: u32) -> Canvas {
            Canvas {
                data: vec![colour; (w * h) as usize],
                w,
                h,
            }
        }
        /*pub(crate) fn set_circle(&mut self, cp: &Point, r: isize, colour: u32) {
            for y in cp.y.saturating_sub(r)..std::cmp::min(self.h as isize, cp.y + r) {
                for x in cp.x.saturating_sub(r)..std::cmp::min(self.w as isize, cp.x + r) {
                    let (dx, dy) = (x.saturating_sub(cp.x), y.saturating_sub(cp.y));
                    if dx * dx + dy * dy < r * r {
                        self.data[(x.saturating_add(y.saturating_mul(self.w as isize))) as usize] =
                            colour;
                    }
                }
            }
        }*/
        pub(crate) fn set_rect(&mut self, p1: &Point, p2: &Point, colour: u32) {
            assert_ne!(p1.y, p2.y);
            assert_ne!(p1.x, p2.x);
            let mut x1 = p1.x;
            let mut y1 = p1.y;
            let mut x2 = p2.x;
            let mut y2 = p2.y;
            if y1 > y2 {
                std::mem::swap(&mut y1, &mut y2);
            }
            if x1 > x2 {
                std::mem::swap(&mut x1, &mut x2);
            }
            for dy in std::cmp::min(y1, (self.h - 1) as isize)..std::cmp::min(y2, self.h as isize) {
                for dx in
                    std::cmp::min(x1, (self.w - 1) as isize)..std::cmp::min(x2, self.w as isize)
                {
                    self.data[(dx as u32 + self.w * dy as u32) as usize] = colour;
                }
            }
        }
        /*pub(crate) fn set_line(&mut self, p1: &Point, p2: &Point, colour: u32) {
            for coord in super::bresenham::Bresenham::new(p1, p2) {
                Canvas::set_pixel(self, &coord, colour);
            }
        }*/
        pub(crate) fn set_pixel(&mut self, p: &Point, colour: u32) {
            self.data[(std::cmp::min(p.x, (self.w - 1) as isize)
                + std::cmp::min(p.y, (self.h - 1) as isize).saturating_mul(self.w as isize))
                as usize] = colour;
        }
        /*pub(crate) fn set_rainbow(c: &mut Canvas) {
            let mut colour: u32 = 0xFF0000FF;
            let mut dc: Dcol = Dcol::Redplusgreen;
            enum Dcol {
                Redplusgreen,
                Minusredgreen,
                Greenplusblue,
                Minusgreenblue,
                Blueplusred,
                Minusbluered,
            }
            for x in 0..c.w {
                c.set_rect(
                    &Point {
                        x: x as isize,
                        y: 0,
                    },
                    &Point {
                        x: 1,
                        y: c.h as isize,
                    },
                    colour,
                );
                match dc {
                    Dcol::Redplusgreen => {
                        colour = colour.saturating_add(0x010000);
                        if colour >> 16 & 0xFF == 0xFF {
                            dc = Dcol::Minusredgreen;
                        }
                    }
                    Dcol::Minusredgreen => {
                        colour = colour.saturating_sub(0x01000000);
                        if colour >> 24 & 0xFF == 0x0 {
                            dc = Dcol::Greenplusblue;
                        }
                    }
                    Dcol::Greenplusblue => {
                        colour = colour.saturating_add(0x0100);
                        if colour >> 8 & 0xFF == 0xFF {
                            dc = Dcol::Minusgreenblue;
                        }
                    }
                    Dcol::Minusgreenblue => {
                        colour = colour.saturating_sub(0x010000);
                        if colour >> 16 & 0xFF == 0x0 {
                            dc = Dcol::Blueplusred;
                        }
                    }
                    Dcol::Blueplusred => {
                        colour = colour.saturating_add(0x01000000);
                        if colour >> 24 & 0xFF == 0xFF {
                            dc = Dcol::Minusbluered;
                        }
                    }
                    Dcol::Minusbluered => {
                        colour = colour.saturating_sub(0x0100);
                        if colour >> 8 & 0xFF == 0x0 {
                            dc = Dcol::Redplusgreen;
                        }
                    }
                }
            }
        }*/
    }
}
/*mod example {
    use crate::bresenham::Bresenham;
    use crate::canvas::{Canvas, Point};
    use crate::png::Chunk;
    use crate::{canvas, u32_to_u8};
    use std::io::Write;

    const DRACVEC: [u32; 7] = [
        0x8BE9FDFF, 0x50fa7bff, 0xffb86cff, 0xff79c6ff, 0xbd93f9ff, 0xff5555ff, 0xf1fa8cff,
    ];

    pub(crate) fn circles(w: u32, h: u32) {
        let mut f: std::fs::File = std::fs::File::create("examples/circles.png").unwrap();
        let mut c: Canvas = Canvas::new(0x505050FF, w, h);
        let r: u32 = std::cmp::min(w.saturating_div(10), h.saturating_div(10)).saturating_div(2);
        let cellw: u32 = w.saturating_div(10);
        let cellh: u32 = h.saturating_div(10);
        Canvas::set_circle(
            &mut c,
            &Point {
                x: r as isize,
                y: r as isize,
            },
            r as isize,
            DRACVEC[rand::Rng::gen_range(&mut rand::thread_rng(), 0..7)],
        );
        Canvas::set_circle(
            &mut c,
            &Point {
                x: (w - r) as isize,
                y: r as isize,
            },
            r as isize,
            DRACVEC[rand::Rng::gen_range(&mut rand::thread_rng(), 0..7)],
        );
        Canvas::set_circle(
            &mut c,
            &Point {
                x: (w - r) as isize,
                y: (h - r) as isize,
            },
            r as isize,
            DRACVEC[rand::Rng::gen_range(&mut rand::thread_rng(), 0..7)],
        );
        Canvas::set_circle(
            &mut c,
            &Point {
                x: r as isize,
                y: (h - r) as isize,
            },
            r as isize,
            DRACVEC[rand::Rng::gen_range(&mut rand::thread_rng(), 0..7)],
        );
        Canvas::set_circle(
            &mut c,
            &Point {
                x: (w.saturating_div(2) - r) as isize,
                y: h.saturating_div(2) as isize,
            },
            r as isize,
            0xFFFFFFFF,
        );
        Canvas::set_circle(
            &mut c,
            &Point {
                x: (w.saturating_div(2) + r) as isize,
                y: h.saturating_div(2) as isize,
            },
            r as isize,
            0xFF,
        );
        f.write_all(&Chunk::form_chunk(Chunk::Sign)).unwrap();
        f.write_all(&Chunk::form_chunk(Chunk::IHDR(c.w, c.h, 8, 6, 0, 0, 0)))
            .unwrap();
        f.write_all(&Chunk::form_chunk(Chunk::IDAT(
            c.w,
            c.h,
            u32_to_u8(&c.data),
        )))
        .unwrap();
        f.write_all(&Chunk::form_chunk(Chunk::IEND)).unwrap();
    }
    pub(crate) fn borders(w: u32, h: u32) {
        let mut f: std::fs::File = std::fs::File::create("examples/borders.png").unwrap();
        let mut c: Canvas = Canvas::new(0x505050FF, w, h);
        Canvas::set_rect(
            &mut c,
            &Point { x: 0, y: 0 },
            &Point {
                x: w as isize,
                y: 1,
            },
            DRACVEC[rand::Rng::gen_range(&mut rand::thread_rng(), 0..7)],
        );
        Canvas::set_rect(
            &mut c,
            &Point { x: 0, y: 0 },
            &Point {
                x: 1,
                y: h as isize,
            },
            DRACVEC[rand::Rng::gen_range(&mut rand::thread_rng(), 0..7)],
        );
        Canvas::set_rect(
            &mut c,
            &Point {
                x: w as isize,
                y: h as isize,
            },
            &Point {
                x: (w - 1) as isize,
                y: 0,
            },
            DRACVEC[rand::Rng::gen_range(&mut rand::thread_rng(), 0..7)],
        );
        Canvas::set_rect(
            &mut c,
            &Point {
                x: w as isize,
                y: h as isize,
            },
            &Point {
                x: 0,
                y: (h - 1) as isize,
            },
            DRACVEC[rand::Rng::gen_range(&mut rand::thread_rng(), 0..7)],
        );
        f.write_all(&Chunk::form_chunk(Chunk::Sign)).unwrap();
        f.write_all(&Chunk::form_chunk(Chunk::IHDR(c.w, c.h, 8, 6, 0, 0, 0)))
            .unwrap();
        f.write_all(&Chunk::form_chunk(Chunk::IDAT(
            c.w,
            c.h,
            u32_to_u8(&c.data),
        )))
        .unwrap();
        f.write_all(&Chunk::form_chunk(Chunk::IEND)).unwrap();
    }
    pub(crate) fn test() {
        let mut f: std::fs::File = std::fs::File::create("examples/pixle.png").unwrap();
        let mut c: Canvas = Canvas::new(0x505050FF, 800, 600);
        c.set_line(
            &Point { x: 100, y: 150 },
            &Point { x: 200, y: 170 },
            0xFF0000FF,
        );
        c.set_line(&Point { x: 100, y: 150 }, &Point { x: 60, y: 200 }, 0xFFFF);
        c.set_line(
            &Point { x: 200, y: 170 },
            &Point { x: 60, y: 200 },
            0xFF00FF,
        );
        Canvas::set_pixel(&mut c, &Point { x: 2, y: 0 }, 0xFFFFFFFF);
        Canvas::set_pixel(&mut c, &Point { x: 1, y: 0 }, 0x808080FF);
        Canvas::set_pixel(&mut c, &Point { x: 0, y: 0 }, 0xFF);
        Canvas::set_pixel(&mut c, &Point { x: 0, y: 1 }, 0xFF0000FF);
        Canvas::set_pixel(&mut c, &Point { x: 1, y: 1 }, 0xFF00FF);
        Canvas::set_pixel(&mut c, &Point { x: 2, y: 1 }, 0xFFFF);
        f.write_all(&Chunk::form_chunk(Chunk::Sign)).unwrap();
        f.write_all(&Chunk::form_chunk(Chunk::IHDR(c.w, c.h, 8, 6, 0, 0, 0)))
            .unwrap();
        f.write_all(&Chunk::form_chunk(Chunk::IDAT(
            c.w,
            c.h,
            u32_to_u8(&c.data),
        )))
        .unwrap();
        f.write_all(&Chunk::form_chunk(Chunk::IEND)).unwrap();
    }
}*/
/*fn get_same_res_divs(a: u32, b: u32) -> Vec<(u32, (u32, u32))> {
    let mut res: Vec<(u32, (u32, u32))> = (1..=b >> 1).fold(Vec::new(), |mut acc, d| {
        if b % d == 0 && a % (b / d) == 0 {
            acc.push((b / d, (a / (b / d), d)));
        }
        acc
    });
    res.sort();
    res
}*/
/*fn get_divs(a: u32) -> Vec<(u32, u32)> {
    let mut res: Vec<(u32, u32)> = (1..=a >> 1).fold(Vec::new(), |mut acc, d| {
        if a % d == 0 {
            acc.push((d, a / d));
        }
        acc
    });
    res.reverse();
    res
}*/
fn u32_to_u8(v: &[u32]) -> Vec<u8> {
    v.iter().fold(Vec::new(), |mut acc, &n| {
        acc.push((n >> 24) as u8);
        acc.push(((n >> 16) & 0xFF) as u8);
        acc.push(((n >> 8) & 0xFF) as u8);
        acc.push((n & 0xFF) as u8);
        acc
    })
}
// 4096 x 2160
// const SMOL_RES: (u32, u32) = (800, 600);
// const MBP_RES: (u32, u32) = (3024, 1964);
// const FOURK_RES: (u32, u32) = (4096, 2160);
// const WIDTH: u32 = MBP_RES.0;
// const HEIGHT: u32 = MBP_RES.1;
// const COLS: u32 = WIDTH.saturating_div(3);
// const ROWS: u32 = HEIGHT.saturating_div(3);
const URAL: &str = "URAL";
const BLINK: &str = "\x1b[5m";
const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREN: &str = "\x1b[32m";
const ORNG: &str = "\x1b[33m";
const ONEUP: &str = "\x1b[1F";
const ERLINE: &str = "\x1b[2K";
const ERTOEND: &str = "\x1b[0K";
use crate::canvas::{Canvas, Point};
use crate::png::Chunk;
use std::io::{stdin, stdout, BufRead, BufReader, Write};

fn read_taxograms(path: &str) -> Result<std::collections::HashMap<char, u32>, std::io::Error> {
    let file: std::fs::File = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut taxmap: std::collections::HashMap<char, u32> = std::collections::HashMap::new();
    reader.lines().for_each(|line| {
        if let Ok(contents) = line {
            if let [chr, num] = contents.split_whitespace().collect::<Vec<&str>>()[..] {
                if let Ok(num) = u32::from_str_radix(num, 16) {
                    taxmap.insert(chr.chars().next().unwrap(), num);
                }
            }
        }
    });
    Ok(taxmap)
}
fn main() {
    // println!("{:?}", get_same_res_divs(WIDTH, HEIGHT));
    // println!("{:?}", get_divs(WIDTH));
    // println!("{:?}", get_divs(HEIGHT));
    // circles(WIDTH, HEIGHT);
    // borders(SMOL_RES.0, SMOL_RES.1);
    // test();
    println!(
        "{}Добро пожаловать в наш переводчик, позволяющий общаться с {BLINK}Ними{RESET}.",
        format!("{ERLINE}{ONEUP}").repeat(50)
    );
    let (mut logwidth, mut logheight): (u32, u32) = (800, 600);
    let mut logogname: String = String::from(URAL);
    let mut dictname: String = logogname.clone();
    let mut taxogramms: std::collections::HashMap<char, u32> = std::collections::HashMap::new();
    let mut input: String = String::new();
    let mut status: String = String::new();
    match read_taxograms(&format!("{dictname}.txt")) {
        Ok(tm) => {
            taxogramms = tm;
            status = format!("Словарь {dictname} загружен.");
        }
        Err(e) => status = format!("Невозможно прочитать словарь таксограмм: {RED}{e}{RESET}"),
    }
    let mut taxdict: Vec<(char, String)> = {
        let mut temp: Vec<(char, String)> = Vec::new();
        for (k, v) in taxogramms.clone() {
            temp.push((k, format!("{:08X}", v)));
        }
        temp.sort();
        temp
    };
    'main: loop {
        taxdict = {
            let mut temp: Vec<(char, String)> = Vec::new();
            for (k, v) in taxogramms.clone() {
                temp.push((k, format!("{:08X}", v)));
            }
            temp.sort();
            temp
        };
        println!(
            "Вы можете следующее:\n\
        0: Указать название логографа; {GREN}{}{RESET}.png\n\
        1: Обновить его таксограммы; {GREN}{}{RESET}\n\
        2: Указать название словаря;\n\
        3: Записать таксограммы в словарь;\n\
        4: Загрузить текущий словарь в память; {GREN}{}{RESET}.txt\n\
        5: Очистить память;\n\
        6: Указать измерения логографа; {GREN}{}{RESET}\n\
        7: Создать логограф;\n\
        8: Запросить значения таксограмм;\n\
        9: Выйти.\n{status}",
            logogname,
            taxdict.iter().fold(String::new(), |mut acc, e| {
                acc.push(e.0);
                acc
            }),
            dictname,
            format_args!("{}x{}", logwidth, logheight)
        );
        status.clear();
        input.clear();
        stdin().read_line(&mut input);
        println!("{ERLINE}{ONEUP}{ERLINE}{ONEUP}",);
        let parsres = input.trim().parse::<u32>();
        input.clear();
        match parsres {
            Ok(0) => {
                println!("{ONEUP}{ERLINE}Введите название логографа. ");
                match stdin().read_line(&mut input) {
                    Ok(_) => {
                        println!("{ONEUP}{ERLINE}{ONEUP}{ERLINE}");
                        logogname = String::from(input.trim());
                        status = format!("{GREN}Смена имени логографа успешна.{RESET}");
                    }
                    Err(e) => status = format!("{e}"),
                }
            }
            Ok(1) => {
                let mut temptgm: std::collections::HashMap<char, u32> =
                    std::collections::HashMap::new();
                'two: loop {
                    input.clear();
                    let tgmstr = {
                        let mut t = temptgm.clone().into_keys().collect::<Vec<char>>();
                        t.sort();
                        t.iter().collect::<String>()
                    };
                    println!("{ONEUP}{ERLINE}Введите {RED}q{RESET} для завершения, {RED}r{RESET} для сброса. Будут обновлены: {GREN}{}{RESET}",{
                    if temptgm.is_empty() {
                        ERTOEND
                    } else {&tgmstr}
                });
                    if stdin().read_line(&mut input).is_ok() {
                        if let [chr, num] = input.split_whitespace().collect::<Vec<&str>>()[..] {
                            if let Ok(num) = u32::from_str_radix(num, 16) {
                                temptgm.insert(chr.chars().next().unwrap(), num);
                            }
                        }
                        if input.trim() == "q" {
                            for (k, v) in temptgm {
                                taxogramms.insert(k, v);
                            }
                            status = format!("{GREN}Записано: {tgmstr}{RESET}");
                            println!("{ERLINE}{ONEUP}{ERLINE}{ONEUP}");
                            break 'two;
                        }
                        if input.trim() == "r" {
                            temptgm.clear();
                        }
                        println!("{ERLINE}{ONEUP}{ERLINE}{ONEUP}");
                    }
                }
            }
            Ok(2) => {
                println!("{ONEUP}{ERLINE}Введите имя словаря.");
                match stdin().read_line(&mut input) {
                    Ok(_) => {
                        println!("{ERLINE}{ONEUP}{ERLINE}{ONEUP}");
                        dictname = String::from(input.trim());
                        status = format!("{GREN}Словарь сменён.{RESET}")
                    }
                    Err(e) => status = format!("Неудалось сменить словарь: {e}"),
                }
            }
            Ok(3) => {
                println!("{ONEUP}{ERLINE}");
                match std::fs::File::create(&format!("{dictname}.txt")) {
                    Ok(mut f) => {
                        println!("{ERLINE}{ONEUP}{ERLINE}{ONEUP}");
                        for (k, v) in taxdict.clone() {
                            write!(f, "{}", format_args!("{k} {v}\n"));
                        }
                        status = format!("{GREN}Изменения в таксограммах записаны.{RESET}");
                    }
                    Err(e) => status = format!("Невозможно записать словарь: {RED}{e}{RESET}"),
                };
            }
            Ok(4) => {
                println! {"{ONEUP}{ERLINE}"};
                match read_taxograms(&format!("{dictname}.txt")) {
                    Ok(tm) => {
                        taxogramms = tm;
                        status = format!("Словарь {dictname} загружен.");
                    }
                    Err(e) => {
                        status = format!("Невозможно прочитать словарь таксограмм: {RED}{e}{RESET}")
                    }
                }
            }
            Ok(5) => {
                println!("{ERLINE}{ONEUP}{ERLINE}{ONEUP}");
                taxogramms.clear();
                status = format!("{GREN}Память таксограмм сброшена.{RESET}");
            }
            Ok(6) => {
                println!("{ONEUP}{ERLINE}Введите измерения логографа.");
                if stdin().read_line(&mut input).is_ok() {
                    if let [width, height] = input.split_whitespace().collect::<Vec<&str>>()[..] {
                        match (width.parse::<u32>(), height.parse::<u32>()) {
                            (Ok(w), Ok(h)) => {
                                println!("{ERLINE}{ONEUP}{ERLINE}{ONEUP}");
                                (logwidth, logheight) = (w, h);
                                status = format!("{GREN}Смена измерений успешна.{RESET}");
                            }
                            (Err(we), Err(he)) => {
                                status =
                                    format!("{RED}Введена некорректная ширина и высота.{RESET}");
                            }
                            (Err(we), _) => {
                                status = format!("{RED}Введена некорректная ширина.{RESET}");
                            }
                            (_, Err(he)) => {
                                status = format!("{RED}Введена некорректная высота.{RESET}");
                            }
                        }
                    }
                }
            }
            Ok(7) => {
                println!("{ONEUP}{ERLINE}Введите таксограммы.");
                match std::fs::File::create(format!("./{logogname}.png")) {
                    Ok(mut f) => {
                        if stdin().read_line(&mut input).is_ok() {
                            let cols = input.trim().chars().count();
                            let colw = logwidth.saturating_div(cols as u32);
                            let mut c: Canvas =
                                Canvas::new(0xFF, (cols * colw as usize) as u32, logheight);
                            let mut taxogc: u32 = 0;
                            input.trim().chars().for_each(|ch| {
                                if taxogramms.contains_key(&ch) {
                                    c.set_rect(
                                        &Point {
                                            x: (colw * taxogc) as isize,
                                            y: 0,
                                        },
                                        &Point {
                                            x: (colw * (taxogc + 1)) as isize,
                                            y: logheight as isize,
                                        },
                                        *taxogramms.get(&ch).unwrap(),
                                    )
                                } else {
                                    for dy in 0..logheight {
                                        for dx in (colw * taxogc)..(colw * (taxogc + 1)) {
                                            c.set_pixel(
                                                &Point {
                                                    x: dx as isize,
                                                    y: dy as isize,
                                                },
                                                rand::Rng::gen_range(
                                                    &mut rand::thread_rng(),
                                                    0xFF..0xFFFFFFFF,
                                                ),
                                            )
                                        }
                                    }
                                }
                                taxogc += 1;
                            });
                            f.write_all(&Chunk::form_chunk(Chunk::Sign));
                            f.write_all(&Chunk::form_chunk(Chunk::IHDR(c.w, c.h, 8, 6, 0, 0, 0)));
                            f.write_all(&Chunk::form_chunk(Chunk::IDAT(
                                c.w,
                                c.h,
                                u32_to_u8(&c.data),
                            )));
                            f.write_all(&Chunk::form_chunk(Chunk::IEND));
                            println!("{ERLINE}{ONEUP}{ERLINE}{ONEUP}");
                            status = format!("{GREN}Логограф создан.{RESET}")
                        }
                    }
                    Err(e) => status = format!("Невозможно создать логограф: {RED}{e}{RESET}"),
                }
            }
            Ok(8) => 'eight: loop {
                input.clear();
                println!(
                    "{ONEUP}{ERLINE}{ONEUP}{ERLINE}Вводите 1 символ за раз или '{RED}done{RESET}' для завершения. {GREN}{status}{RESET}",
                );
                status.clear();
                if stdin().read_line(&mut input).is_ok()
                    && input.trim().chars().count() > 0
                    && input.trim() != "done"
                {
                    status = if !taxogramms.contains_key(&input.chars().next().unwrap()) {
                        format!("{RED}В словаре не найдено.{RESET}")
                    } else {
                        format!(
                            "{} {:08X}",
                            taxogramms
                                .get_key_value(&input.chars().next().unwrap())
                                .unwrap()
                                .0,
                            taxogramms
                                .get_key_value(&input.chars().next().unwrap())
                                .unwrap()
                                .1
                        )
                    }
                } else {
                    println!("{ONEUP}{ERLINE}");
                    break 'eight;
                }
            },
            Ok(9) => break 'main,
            _ => {
                status = String::from("Такого действия нет.");
            }
        }
        println!("{}", format!("{ERLINE}{ONEUP}").repeat(13));
    }
}
