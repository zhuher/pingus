pub(crate) fn u32_to_u8(v: &[u32]) -> Vec<u8> {
    let mut res = Vec::with_capacity(v.len() << 2);
    for &n in v {
        res.extend_from_slice(&[
            (n >> 24) as u8,
            ((n >> 16) & 0xFF) as u8,
            ((n >> 8) & 0xFF) as u8,
            (n & 0xFF) as u8,
        ]);
    }
    res
}
