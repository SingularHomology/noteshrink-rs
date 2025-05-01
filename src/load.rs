use crate::arg::Options;
use crate::processing::{apply_palette, get_palette, sample_pixels};
use crate::save::save;
use crate::types::DPI;
use ndarray::Array3;
use nshare::IntoNdarray3;
use std::error::Error;

pub fn load_img(
    filename: &String,
    options: &Options,
) -> Result<(Array3<u8>, DPI, Vec<Vec<u32>>), Box<dyn Error>> {
    // remove
    // options from the function
    let img = image::open(filename)
        .expect("Couldn't load the image!")
        .into_rgb8();
    let array: Array3<u8> = img.into_ndarray3().permuted_axes([1, 2, 0]);
    let dpi: DPI = DPI { value: 300 };
    let samples = sample_pixels(&array, 5);
    let palette = get_palette(&samples, options);
    let labels = apply_palette(&array, &palette, options);
    let palette = save(
        &(filename.split('.').next().unwrap().to_owned() + "-output.png"),
        labels,
        palette,
        options,
    );

    Ok((array, dpi, palette))
}
