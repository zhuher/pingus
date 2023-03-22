pub(crate) fn u32_to_u8(v: &[u32]) -> Vec<u8> {
    v.iter().fold(Vec::new(), |mut acc, &n| {
        acc.push((n >> 24) as u8);
        acc.push(((n >> 16) & 0xFF) as u8);
        acc.push(((n >> 8) & 0xFF) as u8);
        acc.push((n & 0xFF) as u8);
        acc
    })
}
