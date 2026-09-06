use domain::{ColorRgba, ImagePxRect};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RectShape {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub stroke_color: ColorRgba,
    pub stroke_width: f32,
    pub fill_color: Option<ColorRgba>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EllipseShape {
    pub cx: f32,
    pub cy: f32,
    pub rx: f32,
    pub ry: f32,
    pub stroke_color: ColorRgba,
    pub stroke_width: f32,
    pub fill_color: Option<ColorRgba>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LineShape {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub stroke_color: ColorRgba,
    pub stroke_width: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArrowShape {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub stroke_color: ColorRgba,
    pub stroke_width: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathShape {
    pub points: Vec<(f32, f32)>,
    pub stroke_color: ColorRgba,
    pub stroke_width: f32,
    pub is_highlighter: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextShape {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub font_size: f32,
    pub color: ColorRgba,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MosaicShape {
    pub rect: ImagePxRect,
    pub block_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlurShape {
    pub rect: ImagePxRect,
    pub radius: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepShape {
    pub cx: f32,
    pub cy: f32,
    pub number: u32,
    pub bg_color: ColorRgba,
    pub text_color: ColorRgba,
    pub radius: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnnotationKind {
    Rect(RectShape),
    Ellipse(EllipseShape),
    Line(LineShape),
    Arrow(ArrowShape),
    Freehand(PathShape),
    Text(TextShape),
    Mosaic(MosaicShape),
    Blur(BlurShape),
    Step(StepShape),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnnotationItem {
    pub id: String,
    pub kind: AnnotationKind,
    pub visible: bool,
}

impl AnnotationItem {
    pub fn new(kind: AnnotationKind) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            visible: true,
        }
    }
}
