pub mod arg;
pub mod kmeans_f;
pub mod load;
pub mod processing;
pub mod save;
pub mod types;
pub mod vq;

pub use arg::{parse_arguments, Options};
pub use load::load_img;
pub use processing::{apply_palette, get_bg_color, get_palette, sample_pixels, shrink_image};
pub use save::{adjust_palette, save};
pub use types::DPI;
