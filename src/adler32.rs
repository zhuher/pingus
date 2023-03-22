pub(crate) struct Adler32 {
    a: u32,
    b: u32,
}
const ADLER_MOD: u32 = 0xFFF1;
impl Adler32 {
    pub fn new() -> Adler32 {
        Self { a: 1, b: 0 }
    }
    pub fn update(&mut self, bytes: &[u8]) {
        bytes.iter().for_each(|&i| {
            self.a = (self.a.wrapping_add(u32::from(i))) % ADLER_MOD;
            self.b = (self.a.wrapping_add(self.b)) % ADLER_MOD
        })
    }
    pub fn from(bytes: &[u8]) -> Self {
        let mut temp: Self = Self::new();
        temp.update(bytes);
        temp
    }
    pub fn fin(&self) -> u32 {
        (self.b << 16) | self.a
    }
}
