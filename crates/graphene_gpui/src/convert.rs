use graphene_core::math::{Size2, Vec2};

pub trait Vec2Ext {
    fn to_gpui(self) -> gpui::Point<f32>;
}

impl Vec2Ext for Vec2 {
    fn to_gpui(self) -> gpui::Point<f32> {
        gpui::point(self.x, self.y)
    }
}

pub trait GpuiPointExt {
    fn to_core(self) -> Vec2;
}

impl GpuiPointExt for gpui::Point<f32> {
    fn to_core(self) -> Vec2 {
        Vec2 { x: self.x, y: self.y }
    }
}

pub trait Size2Ext {
    fn to_gpui(self) -> gpui::Size<f32>;
}

impl Size2Ext for Size2 {
    fn to_gpui(self) -> gpui::Size<f32> {
        gpui::size(self.w, self.h)
    }
}

pub trait GpuiSizeExt {
    fn to_core(self) -> Size2;
}

impl GpuiSizeExt for gpui::Size<f32> {
    fn to_core(self) -> Size2 {
        Size2 {
            w: self.width,
            h: self.height,
        }
    }
}
