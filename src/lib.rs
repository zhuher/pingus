#![allow(unused)]
mod adler32;
mod crc32;
mod deflate;
mod helper;

struct Ihdr {
    width: u32,
    height: u32,
    bit_depth: u8,
    colour_type: u8,
    compression_method: u8,
    filter_method: u8,
    interlace_method: u8,
}
struct Idat {
    width: u32,
    height: u32,
    image_data: Vec<u8>,
}
struct Actl {
    num_frames: u32,
    num_plays: u32,
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
struct Fctl {
    sequence_number: u32,
    width: u32,
    height: u32,
    x_offset: u32,
    y_offset: u32,
    delay_num: u16,
    delay_den: u16,
    dispose_op: u8,
    blend_op: u8,
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
struct Fdat {
    width: u32,
    height: u32,
    sequence_number: u32,
    image_data: Vec<u8>,
}
enum Chunk {
    Sign,
    Ihdr(Ihdr),
    Idat(Idat),
    Iend,
    Actl(Actl),
    Fctl(Fctl),
    Fdat(Fdat),
}
impl Chunk {
    fn format(chunk: &[u8]) -> Vec<u8> {
        Vec::from_iter(
            [
                &(chunk[4..].len() as u32).to_be_bytes(),
                chunk,
                &(crc32::Crc32::from(chunk).fin()).to_be_bytes(),
            ]
            .concat(),
        )
    }
    fn form_chunk(self) -> Vec<u8> {
        match self {
            Chunk::Sign => Vec::from(&b"\x89PNG\r\n\x1a\n"[..]),
            Chunk::Ihdr(Ihdr {
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
            Chunk::Idat(Idat {
                width,
                height,
                image_data,
            }) => {
                let width_bytes = width << 2;
                let final_len = (width_bytes + 1) * height;
                let mut chunk_data: Vec<u8> = Vec::with_capacity(final_len as usize);
                let mut window: u32 = (height - 1) * width_bytes;
                loop {
                    chunk_data.push(0);
                    chunk_data.extend_from_slice(
                        &image_data[window as usize..(window + width_bytes) as usize],
                    );
                    if window == 0 {
                        break;
                    }
                    window -= width_bytes;
                }
                assert_eq!(final_len, chunk_data.len() as u32);
                Chunk::format(&Vec::from_iter(
                    [&b"IDAT"[..], &deflate::fake_compress(&chunk_data)[..]].concat(),
                ))
            }
            Chunk::Iend => Chunk::format(&Vec::from(&b"IEND"[..])),
            Chunk::Actl(Actl {
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
            Chunk::Fctl(Fctl {
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
            Chunk::Fdat(Fdat {
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
                    chunk_data.extend_from_slice(
                        &image_data[window as usize..(window + width_byte_4) as usize],
                    );
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
                        &deflate::fake_compress(&chunk_data)[..],
                    ]
                    .concat(),
                ))
            }
        }
    }
}
pub fn create(w: u32, h: u32, data: &[u32], filepath: &str) -> Result<(), std::io::Error> {
    let mut f: std::fs::File = std::fs::File::create(filepath)?;
    std::io::Write::write_all(&mut f, &Chunk::form_chunk(Chunk::Sign))?;
    std::io::Write::write_all(
        &mut f,
        &Chunk::form_chunk(Chunk::Ihdr(Ihdr {
            width: w,
            height: h,
            bit_depth: 8,
            colour_type: 6,
            compression_method: 0,
            filter_method: 0,
            interlace_method: 0,
        })),
    )?;
    std::io::Write::write_all(
        &mut f,
        &Chunk::form_chunk(Chunk::Idat(Idat {
            width: w,
            height: h,
            image_data: helper::u32_to_u8(data),
        })),
    )?;
    std::io::Write::write_all(&mut f, &Chunk::form_chunk(Chunk::Iend))?;
    Ok(())
}
pub fn create_anim(
    width: u32,
    height: u32,
    data: &[Vec<u32>],
    filepath: &str,
) -> Result<(), std::io::Error> {
    let mut f: std::fs::File = std::fs::File::create(filepath)?;
    std::io::Write::write_all(&mut f, &Chunk::form_chunk(Chunk::Sign))?;
    std::io::Write::write_all(
        &mut f,
        &Chunk::form_chunk(Chunk::Ihdr(Ihdr {
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
        &Chunk::form_chunk(Chunk::Actl(Actl {
            num_frames: data.len() as u32,
            num_plays: 0,
        })),
    )?;
    let mut idx: usize = 0;
    for v in data {
        std::io::Write::write_all(
            &mut f,
            &Chunk::form_chunk(Chunk::Fctl(Fctl {
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
                &Chunk::form_chunk(Chunk::Fdat(Fdat {
                    sequence_number: idx as u32,
                    width,
                    height,
                    image_data: helper::u32_to_u8(v),
                })),
            )?;
        } else {
            std::io::Write::write_all(
                &mut f,
                &Chunk::form_chunk(Chunk::Idat(Idat {
                    width,
                    height,
                    image_data: helper::u32_to_u8(v),
                })),
            )?;
        }
        idx += 1;
    }
    std::io::Write::write_all(&mut f, &Chunk::form_chunk(Chunk::Iend))?;
    Ok(())
}
