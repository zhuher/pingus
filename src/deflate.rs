pub fn fake_compress(data: &[u8]) -> Vec<u8> {
    const CHUNK_SIZE: usize = 65530;
    let num_chunks = (data.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;
    let mut raw_data = Vec::with_capacity(2 + num_chunks * 5 + data.len() + 4);
    raw_data.extend_from_slice(&[0x78, 0x1]);
    let mut chunks = data.chunks_exact(CHUNK_SIZE);
    let last_chunk = chunks.remainder();
    for chunk in chunks {
        raw_data.extend_from_slice(&[
            0,
            (chunk.len() & 0xff) as u8,
            ((chunk.len() >> 8) & 0xff) as u8,
            !(chunk.len() as u16) as u8,
            !(chunk.len() as u16 >> 8) as u8,
        ]);
        raw_data.extend_from_slice(chunk);
    }
    if !last_chunk.is_empty() {
        raw_data.extend_from_slice(&[
            1,
            (last_chunk.len() & 0xff) as u8,
            ((last_chunk.len() >> 8) & 0xff) as u8,
            !(last_chunk.len() as u16) as u8,
            !(last_chunk.len() as u16 >> 8) as u8,
        ]);
        raw_data.extend_from_slice(last_chunk);
    }
    raw_data.extend_from_slice(&crate::adler32::Adler32::from(data).fin().to_be_bytes());
    raw_data
}
