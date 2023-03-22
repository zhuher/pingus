pub(crate) struct Crc32 {
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
    ///! This just calls new and update on bytes
    pub(crate) fn from(bytes: &[u8]) -> Crc32 {
        let mut crc: Crc32 = Crc32::new();
        crc.update(bytes);
        crc
    }

    fn update(&mut self, bytes: &[u8]) {
        self.value = bytes.iter().fold(self.value, |acc, &i| {
            self.table[((acc ^ u32::from(i)) & 0xFF) as usize] ^ (acc >> 8)
        });
    }

    pub(crate) fn fin(&mut self) -> u32 {
        self.value ^ 0xFFFFFFFF
    }
}
