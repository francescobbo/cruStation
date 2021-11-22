use super::algebra::Vector3;

#[derive(Copy, Clone, Debug)]
pub(crate) struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub code: u8,
}

impl Color {
    pub fn new() -> Color {
        Color {
            r: 0,
            g: 0,
            b: 0,
            code: 0,
        }
    }

    pub fn from(value: u32) -> Color {
        Color {
            r: value as u8,
            g: (value >> 8) as u8,
            b: (value >> 16) as u8,
            code: (value >> 24) as u8,
        }
    }

    pub fn as_u32(&self) -> u32 {
        self.r as u32
            | ((self.g as u32) << 8)
            | ((self.b as u32) << 16)
            | ((self.code as u32) << 24)
    }

    pub fn as_vec(&self) -> Vector3 {
        Vector3(self.r as i64, self.g as i64, self.b as i64)
    }
}

impl From<Color> for u32 {
    fn from(color: Color) -> u32 {
        color.as_u32()
    }
}
