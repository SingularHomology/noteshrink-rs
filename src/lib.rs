pub mod arg;
pub mod kmeans_f;
pub mod load;
pub mod pdf;
pub mod processing;
pub mod save;
pub mod types;

pub use arg::{parse_arguments, Options};
pub use load::{extract_dpi, get_dpi, load_img};
pub use pdf::{export_pdf, export_pdf_parallel, PdfPage};
pub use processing::{
    apply_palette, get_bg_color, get_palette, sample_pixels, shrink_image, shrink_image_in_memory,
    shrink_preview,
};
pub use save::{adjust_palette, export_images_parallel, save};
pub use types::{Palette, RgbColor, ShrinkParams, DPI};
