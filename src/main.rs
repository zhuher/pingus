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
            for dy in y1..y2 {
                for dx in x1..x2 {
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
            self.data[(p.x + p.y.saturating_mul(self.w as isize)) as usize] = colour;
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
/*fn u32_to_u8(v: &[u32]) -> Vec<u8> {
    v.iter().fold(Vec::new(), |mut acc, &n| {
        acc.push((n >> 24) as u8);
        acc.push(((n >> 16) & 0xFF) as u8);
        acc.push(((n >> 8) & 0xFF) as u8);
        acc.push((n & 0xFF) as u8);
        acc
    })
}*/
// 4096 x 2160
// const SMOL_RES: (u32, u32) = (800, 600);
// const MBP_RES: (u32, u32) = (3024, 1964);
// const FOURK_RES: (u32, u32) = (4096, 2160);
// const WIDTH: u32 = MBP_RES.0;
// const HEIGHT: u32 = MBP_RES.1;
// const COLS: u32 = WIDTH.saturating_div(3);
// const ROWS: u32 = HEIGHT.saturating_div(3);
const URAL: &str = "URAL";
fn main() {
    // println!("{:?}", get_same_res_divs(WIDTH, HEIGHT));
    // println!("{:?}", get_divs(WIDTH));
    // println!("{:?}", get_divs(HEIGHT));
    // circles(WIDTH, HEIGHT);
    // borders(SMOL_RES.0, SMOL_RES.1);
    // test();
    println!("Добро пожаловать в наш переводчик, позволяющий общаться с \x1b[5mНими\x1b[0m.");
}
