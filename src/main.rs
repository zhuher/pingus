// use crate::example::*;
#![allow(unused)]

use crate::example::*;

mod png {
    use std::io::Write;

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
    pub(crate) fn make(c: &super::canvas::Canvas, filepath: &str) {
        let mut f: std::fs::File = std::fs::File::create(filepath).unwrap();
        f.write_all(&Chunk::form_chunk(Chunk::Sign)).unwrap();
        f.write_all(&Chunk::form_chunk(Chunk::IHDR(c.w, c.h, 8, 6, 0, 0, 0)))
            .unwrap();
        f.write_all(&Chunk::form_chunk(Chunk::IDAT(
            c.w,
            c.h,
            super::u32_to_u8(&c.data),
        )))
        .unwrap();
        f.write_all(&Chunk::form_chunk(Chunk::IEND)).unwrap();
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
        let mut adl = super::adler::Adler32::new();
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
macro_rules! set_pixel {
    ($canvas:expr, $point:expr, $colour:expr) => {
        $canvas.data[$point.0 as usize + ($point.1 as usize).saturating_mul($canvas.w as usize)] =
            $colour;
    };
}
macro_rules! rng_range {
    ($range:expr) => {
        rand::Rng::gen_range(&mut rand::thread_rng(), $range)
    };
}
mod canvas {
    #[derive(Debug)]
    pub(crate) struct Canvas {
        pub(crate) data: Vec<u32>,
        pub(crate) w: u32,
        pub(crate) h: u32,
    }
    pub(crate) type Point = (isize, isize);
    impl Canvas {
        pub(crate) fn new(colour: u32, w: u32, h: u32) -> Canvas {
            Canvas {
                data: vec![colour; (w * h) as usize],
                w,
                h,
            }
        }
        pub(crate) fn octantcircle(&mut self, cp: Point, r: isize) {
            for y in cp.1.saturating_sub(r)..std::cmp::min(self.h as isize, cp.1 + r) {
                for x in cp.0.saturating_sub(r)..std::cmp::min(self.w as isize, cp.0 + r) {
                    let (dx, dy) = (x.saturating_sub(cp.0), y.saturating_sub(cp.1));
                    if dx * dx + dy * dy < r * r {
                        set_pixel!(
                            self,
                            (x, y),
                            if x == cp.0 || y == cp.1 || (cp.0 - x).abs() == (cp.1 - y).abs() {
                                0xFF
                            } else {
                                match super::bresenham::octant(cp, (x, y)) {
                                    0 => super::DRRED,
                                    1 => super::DRORANGE,
                                    2 => super::DRYELLOW,
                                    3 => super::DRGREEN,
                                    4 => super::DRCYAN,
                                    5 => super::DRPURPLE,
                                    6 => super::DRPINK,
                                    7 => 0xFFFFFFFF,
                                    _ => unreachable!(),
                                }
                            }
                        );
                    }
                }
            }
        }
        pub(crate) fn set_circle(&mut self, cp: Point, r: isize, colour: u32, rand: bool) {
            for y in
                std::cmp::max(cp.1.saturating_sub(r), 0)..std::cmp::min(self.h as isize, cp.1 + r)
            {
                for x in std::cmp::max(cp.0.saturating_sub(r), 0)
                    ..std::cmp::min(self.w as isize, cp.0 + r)
                {
                    let (dx, dy) = (x.saturating_sub(cp.0), y.saturating_sub(cp.1));
                    if dx * dx + dy * dy < r * r {
                        set_pixel!(
                            self,
                            (x, y),
                            if !rand {
                                colour
                            } else {
                                rng_range!(0xff..=0xffffffff)
                            }
                        );
                    }
                }
            }
        }
        pub(crate) fn set_rect(&mut self, mut p1: Point, mut p2: Point, colour: u32, rand: bool) {
            assert_ne!(p1, p2);
            if p1.0 > p2.0 {
                std::mem::swap(&mut p1.0, &mut p2.0);
            };
            if p1.1 > p2.1 {
                std::mem::swap(&mut p1.1, &mut p2.1);
            };
            if p1.1 == p2.1 {
                p2.1 = p2.0;
            }
            if p1.0 == p2.0 {
                p2.0 = p2.1;
            }
            for y in std::cmp::max(p1.1, 0)..std::cmp::min(p2.1, self.h as isize) {
                for x in std::cmp::max(p1.0, 0)..std::cmp::min(p2.0, self.w as isize) {
                    set_pixel!(
                        self,
                        (x, y),
                        if !rand {
                            colour
                        } else {
                            rng_range!(0xff..=0xffffffff)
                        }
                    );
                }
            }
        }
        pub(crate) fn set_line(&mut self, mut p1: Point, p2: Point, colour: u32, rand: bool) {
            let dx = (p2.0 - p1.0).abs();
            let dy = (p2.1 - p1.1).abs();
            let (sx, sy) = crate::bresenham::octant_to_d(crate::bresenham::octant(p1, p2));
            let mut err = dx - dy;
            loop {
                set_pixel!(
                    self,
                    p1,
                    if !rand {
                        colour
                    } else {
                        rng_range!(0xff..=0xffffffff)
                    }
                );
                if p1.0 == p2.0 && p1.1 == p2.1 {
                    break;
                }
                let e2 = err << 1;
                if e2 > -dy {
                    err -= dy;
                    p1.0 += sx;
                }
                if e2 < dx {
                    err += dx;
                    p1.1 += sy;
                }
            }
        }
        pub(crate) fn set_rainbow(&mut self) {
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
            for x in 0..self.w {
                self.set_rect((x as isize, 0), (1, self.h as isize), colour, false);
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
        }
        pub(crate) fn set_triangle(
            &mut self,
            a: Point,
            b: Point,
            c: Point,
            colour: u32,
            rand: bool,
        ) {
            for y in std::cmp::min(std::cmp::min(a.1, self.h as isize), std::cmp::min(b.1, c.1))
                ..std::cmp::max(a.1, std::cmp::max(b.1, c.1))
            {
                for x in std::cmp::min(std::cmp::min(a.0, self.w as isize), std::cmp::min(b.0, c.0))
                    ..std::cmp::max(a.0, std::cmp::max(b.0, c.0))
                {
                    let thirdtdabpos: bool = (x - a.0) * (b.1 - a.1) - (y - a.1) * (b.0 - a.0) > 0;
                    if thirdtdabpos != ((x - a.0) * (c.1 - a.1) - (y - a.1) * (c.0 - a.0) > 0)
                        && ((x - b.0) * (c.1 - b.1) - (y - b.1) * (c.0 - b.0) > 0) == thirdtdabpos
                    {
                        set_pixel!(
                            self,
                            (x, y),
                            if !rand {
                                colour
                            } else {
                                rng_range!(0xff..=0xffffffff)
                            }
                        );
                    }
                }
            }
        }
    }
}

mod bresenham {
    pub(crate) fn octant(p1: crate::canvas::Point, p2: crate::canvas::Point) -> u8 {
        let (mut dx, mut dy) = (p2.0 - p1.0, p2.1 - p1.1);
        let mut octant = 0;
        if dy < 0 {
            (dx, dy) = (-dx, -dy);
            octant += 4;
        }
        if dx < 0 {
            (dx, dy) = (dy, -dx);
            octant += 2
        }
        if dx < dy {
            octant += 1
        }
        octant
    }
    pub(crate) fn octant_to_d(octant: u8) -> crate::canvas::Point {
        match octant {
            0 | 1 => (1, 1),
            2 | 3 => (-1, 1),
            4 | 5 => (-1, -1),
            6 | 7 => (1, -1),
            _ => unreachable!(),
        }
    }
    // fn _calc(p1: Point, p2: Point) -> Vec<Point> {
    //     Vec::new()
    // }
}
mod example {
    use super::{
        canvas::{Canvas, Point},
        png::{make, Chunk},
        u32_to_u8,
    };
    use std::io::Write;

    pub(crate) fn circles(w: u32, h: u32, columns: isize, rows: isize) {
        let mut c: Canvas = Canvas::new(0x282a36ff, w, h);
        let cellw: isize = (w as isize).saturating_div(columns);
        let cellh: isize = (h as isize).saturating_div(rows);
        let (xpad, ypad) = ((w as isize % columns) >> 1, (h as isize % rows) >> 1);
        let r: isize = std::cmp::min(cellw, cellh) >> 1;
        let (mut x, mut y) = (xpad, ypad);
        loop {
            if y > c.h as isize {
                break;
            }
            loop {
                let cp = ((x + (cellw >> 1)), y + (cellh >> 1));
                if cp.0 > c.w as isize || cp.1 > c.h as isize {
                    break;
                }
                c.set_circle(cp, r, super::DRACVEC[rng_range!(0..7)], false);
                x += cellw;
            }
            x = xpad;
            y += cellh;
        }
        make(&c, "examples/circles.png");
    }
    pub(crate) fn borders(w: u32, h: u32) {
        let mut c: Canvas = Canvas::new(0x282a36ff, w, h);
        Canvas::set_rect(
            &mut c,
            (0, 0),
            (w as isize, 1),
            super::DRACVEC[rng_range!(0..7)],
            false,
        );
        Canvas::set_rect(
            &mut c,
            (0, 0),
            (1, h as isize),
            super::DRACVEC[rng_range!(0..7)],
            false,
        );
        Canvas::set_rect(
            &mut c,
            (w as isize, h as isize),
            ((w - 1) as isize, 0),
            super::DRACVEC[rng_range!(0..7)],
            false,
        );
        Canvas::set_rect(
            &mut c,
            (w as isize, h as isize),
            (0, (h - 1) as isize),
            super::DRACVEC[rng_range!(0..7)],
            false,
        );
        make(&c, "examples/borders.png");
    }
    pub(crate) fn pixles() {
        let mut c: Canvas = Canvas::new(0x282a36ff, 3, 2);
        set_pixel!(c, (2, 0), 0xFFFFFFFF);
        set_pixel!(c, (1, 0), 0x808080FF);
        set_pixel!(c, (0, 0), 0xFF);
        set_pixel!(c, (0, 1), 0xFF0000FF);
        set_pixel!(c, (1, 1), 0xFF00FF);
        set_pixel!(c, (2, 1), 0xFFFF);
        make(&c, "examples/pixles.png");
    }
    pub(crate) fn octant_circle(w: u32, h: u32, cp: Point, r: isize) {
        let mut c: Canvas = Canvas::new(0x282a36ff, w, h);
        Canvas::octantcircle(&mut c, cp, r);
        make(&c, "examples/octant_circle.png");
    }
    pub(crate) fn lines(w: u32, h: u32, divideby: isize) {
        let mut c: Canvas = Canvas::new(0x282a36ff, w, h);
        let mut points: Vec<Point> = Vec::new();
        let mut y: isize = 0;
        loop {
            if y > h as isize - 1 {
                y = h as isize - 1;
                if points.last().unwrap().1 != h as isize - 1 {
                    let mut x: isize = 0;
                    loop {
                        if x > w as isize - 1 {
                            if !points.contains(&(w as isize - 1, y)) {
                                points.push((w as isize - 1, y));
                            }
                            break;
                        }
                        points.push((x, y));
                        x += (w as isize).saturating_div(divideby);
                    }
                }
                break;
            }
            let mut x: isize = 0;
            loop {
                if x > w as isize - 1 {
                    if !points.contains(&(w as isize - 1, y)) {
                        points.push((w as isize - 1, y));
                    }
                    break;
                }
                points.push((x, y));
                x += (w as isize).saturating_div(divideby);
            }
            y += (h as isize).saturating_div(divideby);
        }
        for p1 in &points {
            for p2 in &points {
                if p1 != p2 {
                    Canvas::set_line(&mut c, *p1, *p2, super::DRACVEC[rng_range!(0..7)], false);
                }
            }
        }
        make(&c, "examples/lines.png");
    }
    pub(crate) fn test(w: u32, h: u32) {
        let mut c: Canvas = Canvas::new(0x282a36ff, w, h);
        c.set_triangle(
            (rng_range!(0..(w as isize)), rng_range!(0..(h as isize))),
            (rng_range!(0..(w as isize)), rng_range!(0..(h as isize))),
            (rng_range!(0..(w as isize)), rng_range!(0..(h as isize))),
            rng_range!(0xff..=0xffffffff),
            false,
        );
        make(&c, "examples/test.png");
    }
}
fn get_same_res_divs(a: u32, b: u32) -> Vec<(u32, (u32, u32))> {
    let mut res: Vec<(u32, (u32, u32))> = (1..=b >> 1).fold(Vec::new(), |mut acc, d| {
        if b % d == 0 && a % (b / d) == 0 {
            acc.push((b / d, (a / (b / d), d)));
        }
        acc
    });
    res.sort();
    res
}
fn get_divs(a: u32) -> Vec<(u32, u32)> {
    let mut res: Vec<(u32, u32)> = (1..=a >> 1).fold(Vec::new(), |mut acc, d| {
        if a % d == 0 {
            acc.push((d, a / d));
        }
        acc
    });
    res.reverse();
    res
}
fn u32_to_u8(v: &[u32]) -> Vec<u8> {
    v.iter().fold(Vec::new(), |mut acc, &n| {
        acc.push((n >> 24) as u8);
        acc.push(((n >> 16) & 0xFF) as u8);
        acc.push(((n >> 8) & 0xFF) as u8);
        acc.push((n & 0xFF) as u8);
        acc
    })
}
const DRRED: u32 = 0xFF5555FF;
const DRORANGE: u32 = 0xFFB86CFF;
const DRYELLOW: u32 = 0xF1FA8CFF;
const DRGREEN: u32 = 0x50FA7BFF;
const DRPURPLE: u32 = 0xBD93F9FF;
const DRCYAN: u32 = 0x8BE9FDFF;
const DRPINK: u32 = 0xFF79C6FF;
const DRACVEC: [u32; 7] = [DRRED, DRORANGE, DRYELLOW, DRGREEN, DRCYAN, DRPURPLE, DRPINK];
// 4096 x 2160
const SMOL_RES: (u32, u32) = (800, 600);
const MBP_RES: (u32, u32) = (3024, 1964);
const FOK_RES: (u32, u32) = (4096, 2160);
const WIDTH: u32 = MBP_RES.0;
const HEIGHT: u32 = MBP_RES.1;
const COLS: u32 = WIDTH.saturating_div(3);
const ROWS: u32 = HEIGHT.saturating_div(3);
fn main() {
    // println!("{:?}", get_same_res_divs(WIDTH, HEIGHT));
    // println!("{:?}", get_divs(WIDTH));
    // println!("{:?}", get_divs(HEIGHT));
    // pixles();
    // octant_circle(FOK_RES.0, FOK_RES.1, ((FOK_RES.0 >> 1) as isize, (FOK_RES.1 >> 1) as isize), 400,);
    // borders(800, 600);
    circles(FOK_RES.0, FOK_RES.1, 80, 50);
    // lines(4000, 4000, 10);
    test(64, 64);
    // circles(WIDTH, HEIGHT);
    // borders(SMOL_RES.0, SMOL_RES.1);
    // test();
}
