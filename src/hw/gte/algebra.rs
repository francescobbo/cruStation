use std::num::Wrapping;
use std::ops;

pub enum Axis {
    X,
    Y,
    Z,
}

use Axis::*;

#[derive(Copy, Clone, Debug)]
pub struct Vector3(pub i32, pub i32, pub i32);

impl Vector3 {
    pub fn new() -> Vector3 {
        Vector3(0, 0, 0)
    }

    // pub fn neg(&self) -> Vector3 {
    //     Vector3(
    //         -self.0,
    //         -self.1,
    //         -self.2
    //     )
    // }

    pub fn shift_fraction(&self) -> Vector3 {
        Vector3(self.0 >> 12, self.1 >> 12, self.2 >> 12)
    }

    pub fn cross(&self, other: &Vector3) -> Vector3 {
        Vector3(
            self.z_i32() * other.y_i32() - self.y_i32() * other.z_i32(),
            self.x_i32() * other.z_i32() - self.z_i32() * other.x_i32(),
            self.y_i32() * other.x_i32() - self.x_i32() * other.y_i32(),
        )
    }

    // pub fn x_i16(&self) -> i16 {
    //     self.0 as i16
    // }

    // pub fn x_u16(&self) -> u16 {
    //     self.0 as u16
    // }

    pub fn x_u32(&self) -> u32 {
        self.0 as u16 as u32
    }

    pub fn x_i32(&self) -> i32 {
        self.0 as i32
    }

    pub fn x_u32s(&self) -> u32 {
        self.0 as i32 as u32
    }

    // pub fn y_i16(&self) -> i16 {
    //     self.1 as i16
    // }

    // pub fn y_u16(&self) -> u16 {
    //     self.1 as u16
    // }

    pub fn y_u32(&self) -> u32 {
        self.1 as u16 as u32
    }

    pub fn y_i32(&self) -> i32 {
        self.1 as i32
    }

    pub fn y_u32s(&self) -> u32 {
        self.1 as i32 as u32
    }

    // pub fn z_i16(&self) -> i16 {
    //     self.2 as i16
    // }

    // pub fn z_u16(&self) -> u16 {
    //     self.2 as u16
    // }

    pub fn z_u32(&self) -> u32 {
        self.2 as u16 as u32
    }

    pub fn z_i32(&self) -> i32 {
        self.2 as i32
    }

    pub fn z_u32s(&self) -> u32 {
        self.2 as i32 as u32
    }
}

impl ops::Index<Axis> for Vector3 {
    type Output = i32;

    fn index(&self, index: Axis) -> &Self::Output {
        match index {
            X => &self.0,
            Y => &self.1,
            Z => &self.2,
        }
    }
}

impl ops::IndexMut<Axis> for Vector3 {
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
        match index {
            X => &mut self.0,
            Y => &mut self.1,
            Z => &mut self.2,
        }
    }
}

impl ops::Add for Vector3 {
    type Output = Vector3;

    fn add(self, other: Vector3) -> Self::Output {
        Vector3(self.0 + other.0, self.1 + other.1, self.2 + other.2)
    }
}

impl ops::Sub for Vector3 {
    type Output = Vector3;

    fn sub(self, other: Vector3) -> Self::Output {
        Vector3(self.0 - other.0, self.1 - other.1, self.2 - other.2)
    }
}

impl ops::Mul<i32> for Vector3 {
    type Output = Vector3;

    fn mul(self, scalar: i32) -> Self::Output {
        Vector3(
            (Wrapping(self.0) * Wrapping(scalar)).0,
            (Wrapping(self.1) * Wrapping(scalar)).0,
            (Wrapping(self.2) * Wrapping(scalar)).0,
        )
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Matrix3 {
    f: [Vector3; 3],
}

impl Matrix3 {
    pub fn new() -> Matrix3 {
        Matrix3 {
            f: [Vector3::new(); 3],
        }
    }

    pub fn diagonal(&self) -> Vector3 {
        Vector3(self[0][X], self[1][Y], self[2][Z])
    }
}

impl ops::Index<usize> for Matrix3 {
    type Output = Vector3;

    fn index(&self, index: usize) -> &Self::Output {
        &self.f[index]
    }
}

impl ops::IndexMut<usize> for Matrix3 {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.f[index]
    }
}

impl ops::Mul<Vector3> for Matrix3 {
    type Output = Vector3;

    fn mul(self, vec: Vector3) -> Self::Output {
        Vector3(
            self[0][X] * vec[X] + self[0][Y] * vec[Y] + self[0][Z] * vec[Z],
            self[1][X] * vec[X] + self[1][Y] * vec[Y] + self[1][Z] * vec[Z],
            self[2][X] * vec[X] + self[2][Y] * vec[Y] + self[2][Z] * vec[Z]
        )
    }
}
