pub mod command;
pub mod rasterizer;
pub mod shape;

pub use command::{AnnotationCommand, CommandStack};
pub use rasterizer::composite_annotations;
pub use shape::{
    AnnotationItem, AnnotationKind, ArrowShape, BlurShape, EllipseShape, LineShape, MosaicShape,
    PathShape, RectShape, StepShape, TextShape,
};
