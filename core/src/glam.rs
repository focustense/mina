/// Support for the Glam library. Adds [Lerp] trait implementations for vector types.
use crate::interpolation::Lerp;
use glam::{
    DQuat, DVec2, DVec3, DVec4, I64Vec2, I64Vec3, I64Vec4, IVec2, IVec3, IVec4, Quat, U64Vec2,
    U64Vec3, U64Vec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec3A, Vec4,
};

macro_rules! impl_lerp2 {
    ($($t:ty),*) => {
        $( impl Lerp for $t {
            fn lerp(&self, b: &Self, t: f32) -> Self {
                Self::new(self.x.lerp(&b.x, t), self.y.lerp(&b.y, t))
            }
        }) *
    }
}

macro_rules! impl_lerp3 {
    ($($t:ty),*) => {
        $( impl Lerp for $t {
            fn lerp(&self, b: &Self, t: f32) -> Self {
                Self::new(self.x.lerp(&b.x, t), self.y.lerp(&b.y, t), self.z.lerp(&b.z, t))
            }
        }) *
    }
}

macro_rules! impl_lerp4 {
    ($($t:ty),*) => {
        $( impl Lerp for $t {
            fn lerp(&self, b: &Self, t: f32) -> Self {
                Self::new(self.x.lerp(&b.x, t), self.y.lerp(&b.y, t), self.z.lerp(&b.z, t), self.w.lerp(&b.w, t))
            }
        }) *
    }
}

impl_lerp2! { Vec2, DVec2, IVec2, I64Vec2, UVec2, U64Vec2 }
impl_lerp3! { Vec3, Vec3A, DVec3, IVec3, I64Vec3, UVec3, U64Vec3 }
impl_lerp4! { Vec4, DVec4, IVec4, I64Vec4, UVec4, U64Vec4 }

impl Lerp for Quat {
    fn lerp(&self, y1: &Self, x: f32) -> Self {
        Quat::lerp(*self, *y1, x)
    }
}

impl Lerp for DQuat {
    fn lerp(&self, y1: &Self, x: f32) -> Self {
        DQuat::lerp(*self, *y1, x as f64)
    }
}
