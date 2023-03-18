// use crate::example::*;
#![allow(unused)]

use crate::example::*;

mod png {
    pub(crate) struct IHDR {
        pub(crate) width: u32,
        pub(crate) height: u32,
        pub(crate) bit_depth: u8,
        pub(crate) colour_type: u8,
        pub(crate) compression_method: u8,
        pub(crate) filter_method: u8,
        pub(crate) interlace_method: u8,
    }
    pub(crate) struct IDAT {
        pub(crate) width: u32,
        pub(crate) height: u32,
        pub(crate) image_data: Vec<u8>,
    }
    pub(crate) struct acTL {
        pub(crate) num_frames: u32,
        pub(crate) num_plays: u32,
    }
    //   byte
    //     0    sequence_number       (unsigned int)   Sequence number of the animation chunk, starting from 0
    //     4    width                 (unsigned int)   Width of the following frame
    //     8    height                (unsigned int)   Height of the following frame
    //    12    x_offset              (unsigned int)   X position at which to render the following frame
    //    16    y_offset              (unsigned int)   Y position at which to render the following frame
    //    20    delay_num             (unsigned short) Frame delay fraction numerator
    //    22    delay_den             (unsigned short) Frame delay fraction denominator
    //    24    dispose_op            (byte)           Type of frame area disposal to be done after rendering this frame
    //    25    blend_op              (byte)           Type of frame area rendering for this frame
    pub(crate) struct fcTL {
        pub(crate) sequence_number: u32,
        pub(crate) width: u32,
        pub(crate) height: u32,
        pub(crate) x_offset: u32,
        pub(crate) y_offset: u32,
        pub(crate) delay_num: u16,
        pub(crate) delay_den: u16,
        pub(crate) dispose_op: u8,
        pub(crate) blend_op: u8,
    }
    //   value
    //    0           APNG_DISPOSE_OP_NONE
    //    1           APNG_DISPOSE_OP_BACKGROUND
    //    2           APNG_DISPOSE_OP_PREVIOUS
    // APNG_DISPOSE_OP_NONE: no disposal is done on this frame before rendering the next; the contents of the output buffer are left as is.
    // APNG_DISPOSE_OP_BACKGROUND: the frame's region of the output buffer is to be cleared to fully transparent black before rendering the next frame.
    // APNG_DISPOSE_OP_PREVIOUS: the frame's region of the output buffer is to be reverted to the previous contents before rendering the next frame.
    // If `blend_op` is APNG_BLEND_OP_SOURCE all color components of the frame, including alpha, overwrite the current contents of the frame's output buffer region.
    // If `blend_op` is APNG_BLEND_OP_OVER the frame should be composited onto the output buffer based on its alpha, using a simple OVER operation as described in the "Alpha Channel Processing" section of the PNG specification [PNG-1.2]. Note that the second variation of the sample code is applicable.
    //
    // Note that for the first frame the two blend modes are functionally equivalent due to the clearing of the output buffer at the beginning of each play.
    pub(crate) struct fdAT {
        pub(crate) width: u32,
        pub(crate) height: u32,
        pub(crate) sequence_number: u32,
        pub(crate) image_data: Vec<u8>,
    }
    pub(crate) enum Chunk {
        Sign,
        IHDR(IHDR),
        IDAT(IDAT),
        IEND,
        acTL(acTL),
        fcTL(fcTL),
        fdAT(fdAT),
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
                Chunk::IHDR(IHDR {
                    width,
                    height,
                    bit_depth,
                    colour_type,
                    compression_method,
                    filter_method,
                    interlace_method,
                }) => match (bit_depth, colour_type) {
                    (8, 6) => Chunk::format(&Vec::from_iter(
                        [
                            &b"IHDR"[..],
                            &width.to_be_bytes(),
                            &height.to_be_bytes(),
                            &[
                                bit_depth,
                                colour_type,
                                compression_method,
                                filter_method,
                                interlace_method,
                            ],
                        ]
                        .concat(),
                    )),
                    (_, _) => todo!(),
                },
                Chunk::IDAT(IDAT {
                    width,
                    height,
                    image_data,
                }) => {
                    let width_byte_4 = width << 2;
                    let final_len = (width_byte_4 + 1) * height;
                    let mut chunk_data: Vec<u8> = Vec::with_capacity(final_len as usize);
                    let mut window: u32 = (height - 1) * width_byte_4;
                    loop {
                        chunk_data.push(0);
                        chunk_data
                            .extend(&image_data[window as usize..(window + width_byte_4) as usize]);
                        if window == 0 {
                            break;
                        }
                        window -= width_byte_4;
                    }
                    assert_eq!(final_len, chunk_data.len() as u32);
                    Chunk::format(&Vec::from_iter(
                        [&b"IDAT"[..], &super::nondeflate::compress(&chunk_data)[..]].concat(),
                    ))
                }
                Chunk::IEND => Chunk::format(&Vec::from(&b"IEND"[..])),
                Chunk::acTL(acTL {
                    num_plays,
                    num_frames,
                }) => Chunk::format(&Vec::from_iter(
                    [
                        &b"acTL"[..],
                        &num_frames.to_be_bytes(),
                        &num_plays.to_be_bytes(),
                    ]
                    .concat(),
                )),
                Chunk::fcTL(fcTL {
                    sequence_number,
                    width,
                    height,
                    x_offset,
                    y_offset,
                    delay_num,
                    delay_den,
                    dispose_op,
                    blend_op,
                }) => Chunk::format(&Vec::from_iter(
                    [
                        &b"fcTL"[..],
                        &sequence_number.to_be_bytes(),
                        &width.to_be_bytes(),
                        &height.to_be_bytes(),
                        &x_offset.to_be_bytes(),
                        &y_offset.to_be_bytes(),
                        &delay_num.to_be_bytes(),
                        &delay_den.to_be_bytes(),
                        &[dispose_op, blend_op],
                    ]
                    .concat(),
                )),
                Chunk::fdAT(fdAT {
                    width,
                    height,
                    sequence_number,
                    image_data,
                }) => {
                    let width_byte_4 = width << 2;
                    let final_len = (width_byte_4 + 1) * height;
                    let mut chunk_data: Vec<u8> = Vec::with_capacity(final_len as usize);
                    let mut window: u32 = (height - 1) * width_byte_4;
                    loop {
                        chunk_data.push(0);
                        chunk_data
                            .extend(&image_data[window as usize..(window + width_byte_4) as usize]);
                        if window == 0 {
                            break;
                        }
                        window -= width_byte_4;
                    }
                    assert_eq!(final_len, chunk_data.len() as u32);
                    Chunk::format(&Vec::from_iter(
                        [
                            &b"fdAT"[..],
                            &sequence_number.to_be_bytes(),
                            &super::nondeflate::compress(&chunk_data)[..],
                        ]
                        .concat(),
                    ))
                }
            }
        }
    }
    pub(crate) fn make(
        c: &super::canvas::Canvas,
        filepath: &str,
    ) -> std::result::Result<(), std::io::Error> {
        let mut f: std::fs::File = std::fs::File::create(filepath)?;
        std::io::Write::write_all(&mut f, &Chunk::form_chunk(Chunk::Sign))?;
        std::io::Write::write_all(
            &mut f,
            &Chunk::form_chunk(Chunk::IHDR(IHDR {
                width: c.w,
                height: c.h,
                bit_depth: 8,
                colour_type: 6,
                compression_method: 0,
                filter_method: 0,
                interlace_method: 0,
            })),
        )?;
        std::io::Write::write_all(
            &mut f,
            &Chunk::form_chunk(Chunk::IDAT(IDAT {
                width: c.w,
                height: c.h,
                image_data: super::u32_to_u8(&c.data),
            })),
        )?;
        std::io::Write::write_all(&mut f, &Chunk::form_chunk(Chunk::IEND))?;
        Ok(())
    }
    pub(crate) fn make_animated(
        width: u32,
        height: u32,
        data: Vec<Vec<u32>>,
        filepath: &str,
    ) -> std::result::Result<(), std::io::Error> {
        let mut f: std::fs::File = std::fs::File::create(filepath)?;
        std::io::Write::write_all(&mut f, &Chunk::form_chunk(Chunk::Sign))?;
        std::io::Write::write_all(
            &mut f,
            &Chunk::form_chunk(Chunk::IHDR(IHDR {
                width,
                height,
                colour_type: 6,
                bit_depth: 8,
                compression_method: 0,
                filter_method: 0,
                interlace_method: 0,
            })),
        )?;
        std::io::Write::write_all(
            &mut f,
            &Chunk::form_chunk(Chunk::acTL(acTL {
                num_frames: data.len() as u32,
                num_plays: 0,
            })),
        )?;
        let mut idx: usize = 0;
        for v in data {
            std::io::Write::write_all(
                &mut f,
                &Chunk::form_chunk(Chunk::fcTL(fcTL {
                    sequence_number: idx as u32,
                    width,
                    height,
                    x_offset: 0,
                    y_offset: 0,
                    delay_num: 0,
                    delay_den: 0,
                    dispose_op: 0,
                    blend_op: 0,
                })),
            )?;
            if idx != 0 {
                idx += 1;
                std::io::Write::write_all(
                    &mut f,
                    &Chunk::form_chunk(Chunk::fdAT(fdAT {
                        sequence_number: idx as u32,
                        width,
                        height,
                        image_data: super::u32_to_u8(&v),
                    })),
                )?;
            } else {
                std::io::Write::write_all(
                    &mut f,
                    &Chunk::form_chunk(Chunk::IDAT(IDAT {
                        width,
                        height,
                        image_data: super::u32_to_u8(&v),
                    })),
                )?;
            }
            idx += 1;
        }
        std::io::Write::write_all(&mut f, &Chunk::form_chunk(Chunk::IEND))?;
        Ok(())
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
macro_rules! manhattan_dist {
    ($a:expr, $b:expr) => {
        ($b.0 as isize - $a.0 as isize).abs() + ($b.1 as isize - $a.1 as isize).abs()
    };
}
macro_rules! euclid_dist {
    ($a:expr, $b:expr) => {
        ((($b.0 as isize - $a.0 as isize).abs() * ($b.0 as isize - $a.0 as isize).abs()
            + ($b.1 as isize - $a.1 as isize) * ($b.1 as isize - $a.1 as isize)) as f64)
            .sqrt()
    };
}
mod canvas {
    #[derive(Debug)]
    pub(crate) struct Canvas {
        pub(crate) data: Vec<u32>,
        pub(crate) w: u32,
        pub(crate) h: u32,
        pub(crate) stride: u32,
    }
    pub(crate) type Point = (isize, isize);
    impl Canvas {
        pub(crate) fn new(colour: u32, w: u32, h: u32) -> Canvas {
            Canvas {
                data: vec![colour; (w * h) as usize],
                w,
                h,
                stride: w,
            }
        }
        pub(crate) fn fill(&mut self, colour: u32) {
            self.data = vec![colour; self.w as usize * self.h as usize];
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
        pub(crate) fn set_circle(
            &mut self,
            cp: Point,
            r: isize,
            colour: u32,
            randc: bool,
            randr: bool,
        ) {
            let r = if !randr { r } else { rng_range!(1..=r) };
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
                            if !randc {
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
            let mut counter: u32 = 0;
            let mut dc: Dcol = Dcol::Redplusgreen;
            pub(crate) enum Dcol {
                Redplusgreen,
                Minusredgreen,
                Greenplusblue,
                Minusgreenblue,
                Blueplusred,
                Minusbluered,
            }
            for x in 0..self.w {
                self.set_rect(
                    (x as isize, 0),
                    (x as isize + 1, self.h as isize),
                    colour,
                    false,
                );
                match dc {
                    Dcol::Redplusgreen => {
                        colour = colour.saturating_add(0x010000);
                        counter += 1;
                        if colour >> 16 & 0xFF == 0xFF {
                            dc = Dcol::Minusredgreen;
                        }
                    }
                    Dcol::Minusredgreen => {
                        colour = colour.saturating_sub(0x01000000);
                        counter += 1;
                        if colour >> 24 & 0xFF == 0x0 {
                            dc = Dcol::Greenplusblue;
                        }
                    }
                    Dcol::Greenplusblue => {
                        colour = colour.saturating_add(0x0100);
                        counter += 1;
                        if colour >> 8 & 0xFF == 0xFF {
                            dc = Dcol::Minusgreenblue;
                        }
                    }
                    Dcol::Minusgreenblue => {
                        colour = colour.saturating_sub(0x010000);
                        counter += 1;
                        if colour >> 16 & 0xFF == 0x0 {
                            dc = Dcol::Blueplusred;
                        }
                    }
                    Dcol::Blueplusred => {
                        colour = colour.saturating_add(0x01000000);
                        counter += 1;
                        if colour >> 24 & 0xFF == 0xFF {
                            dc = Dcol::Minusbluered;
                        }
                    }
                    Dcol::Minusbluered => {
                        colour = colour.saturating_sub(0x0100);
                        if colour >> 8 & 0xFF == 0x0 {
                            println!("{counter}");
                            dc = Dcol::Redplusgreen;
                        } else {
                            counter += 1;
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
        pub(crate) fn voronoi(&mut self, points: &[(Point, u32)], distf: &str) {
            for y in 0..self.h {
                for x in 0..self.w {
                    let mut min = isize::MAX;
                    let mut colour: u32 = 0xFF;
                    match distf {
                        "m" => {
                            for point in points {
                                match manhattan_dist!((x, y), point.0).partial_cmp(&min) {
                                    Some(std::cmp::Ordering::Less) => {
                                        min = manhattan_dist!((x, y), point.0);
                                        colour = point.1;
                                    }
                                    Some(std::cmp::Ordering::Equal) => {
                                        colour = point.1;
                                        // println!("x: {x}, y: {y}");
                                    }
                                    _ => {}
                                }
                            }
                        }
                        "e" => {
                            for point in points {
                                match (euclid_dist!((x, y), point.0) as isize).partial_cmp(&min) {
                                    Some(std::cmp::Ordering::Less) => {
                                        min = euclid_dist!((x, y), point.0) as isize;
                                        colour = point.1;
                                    }
                                    Some(std::cmp::Ordering::Equal) => {
                                        colour = point.1;
                                        // println!("x: {x}, y: {y}");
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => return,
                    }
                    set_pixel!(self, (x, y), colour);
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
        png::{acTL, fcTL, fdAT, make, Chunk, IDAT, IHDR},
        u32_to_u8,
    };
    use crate::png::make_animated;
    use std::io::Write;

    pub(crate) fn circles(
        w: u32,
        h: u32,
        columns: isize,
        rows: isize,
        randc: bool,
        randr: bool,
        filename: &str,
    ) {
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
                if cp.0 + r > c.w as isize || cp.1 + r > c.h as isize {
                    break;
                }
                c.set_circle(cp, r, super::DRACVEC[rng_range!(0..7)], randc, randr);
                x += cellw;
            }
            x = xpad;
            y += cellh;
        }
        make(&c, &format!("examples/{filename}.png"));
    }
    pub(crate) fn borders(w: u32, h: u32, filename: &str) {
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
    pub(crate) fn pixles(filename: &str) {
        let mut c: Canvas = Canvas::new(0x282a36ff, 3, 2);
        set_pixel!(c, (2, 0), 0xFFFFFFFF);
        set_pixel!(c, (1, 0), 0x808080FF);
        set_pixel!(c, (0, 0), 0xFF);
        set_pixel!(c, (0, 1), 0xFF0000FF);
        set_pixel!(c, (1, 1), 0xFF00FF);
        set_pixel!(c, (2, 1), 0xFFFF);
        make(&c, &format!("examples/{filename}.png"));
    }
    pub(crate) fn octant_circle(w: u32, h: u32, cp: Point, r: isize, filename: &str) {
        let mut c: Canvas = Canvas::new(0x282a36ff, w, h);
        Canvas::octantcircle(&mut c, cp, r);
        make(&c, &format!("examples/{filename}.png"));
    }
    pub(crate) fn lines(w: u32, h: u32, divideby: isize, colour: u32, filename: &str) {
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
                    Canvas::set_line(
                        &mut c, *p1, *p2, colour, /*super::DRACVEC[rng_range!(0..7)]*/
                        false,
                    );
                }
            }
        }
        make(&c, &format!("examples/{filename}.png"));
    }
    pub(crate) fn bouncy_circle_anim(w: u32, h: u32, r: isize, frames: u32, filename: &str) {
        let mut data: Vec<Vec<u32>> = Vec::with_capacity(frames as usize);
        let mut c: Canvas = Canvas::new(0x282a36ff, w, h);
        let mut x = r;
        let s = ((w - ((x as u32) << 1)) << 1) / frames;
        c.set_circle((x, h as isize >> 1), r, 0xffc0cbff, false, false);
        data.push(c.data.clone());
        for i in 0..(frames >> 1) {
            x += s as isize;
            c.fill(0x282a36ff);
            c.set_circle((x, h as isize >> 1), r, 0xffc0cbff, false, false);
            data.push(c.data.clone());
        }
        let mut temp = data.clone();
        temp.pop();
        temp.reverse();
        temp.pop();
        data.extend(temp);
        make_animated(w, h, data, &format!("examples/{filename}.png")).unwrap();
    }
    pub(crate) fn test(w: u32, h: u32, points: &[(Point, u32)], distf: &str, filename: &str) {
        let mut c: Canvas = Canvas::new(0x282a36ff, w, h);
        c.voronoi(points, distf);
        make(&c, &format!("examples/{filename}.png")).unwrap();
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
const DRBG: u32 = 0x282A36FF;
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
    // circles(799, 599, 8, 6, false, true, "testcirc" /* &str */);
    // lines(5000, 5000, 7, DRGREEN, "lines1");
    let w = 800;
    let h = 600;
    // let v = Vec::from([((100, 100), DRRED), ((200, 300), DRGREEN), ((400, 580), DRPURPLE), ((400, 400), DRORANGE), ((600, 240), DRPINK), ((650, 180), DRYELLOW), ]);
    let rv = (0..10).fold(Vec::with_capacity(10), |mut acc, _| {
        acc.push((
            (rng_range!(0..w) as isize, rng_range!(0..h) as isize),
            rng_range!(0xFF..=0xFFFFFFFF),
        ));
        acc
    });
    println!("{rv:?}");
    test(w, h, &rv, "m", "voronoi_m_test");
    test(w, h, &rv, "e", "voronoi_e_test");

    let a: [[u8; 4]; 4] = [[4; 4]; 4];
    let b: [u8; 16] = [4; 16];
    // circles(WIDTH, HEIGHT);
    // borders(SMOL_RES.0, SMOL_RES.1);
    // test();
}
