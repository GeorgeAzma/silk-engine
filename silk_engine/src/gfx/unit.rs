#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Unit {
    /// pixels
    Px(i32),
    /// 1.0 is min(width, height) pixels
    Mn(f32),
    /// 1.0 is max(width, height) pixels
    Mx(f32),
    /// screen is 0-1 range
    Pc(f32),
}
