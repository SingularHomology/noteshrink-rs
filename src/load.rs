use crate::arg::Options;
use crate::pdf::PdfPage;
use crate::processing::{apply_palette, get_palette, sample_pixels};
use crate::save::{adjust_palette, save};
use crate::types::DPI;
use ndarray::{Array2, Array3};
use std::error::Error;
use std::fs::File;
use std::io::Read;

pub fn extract_dpi(filename: &str) -> Option<DPI> {
    let mut file = File::open(filename).ok()?;
    let mut header = [0u8; 1024];
    let bytes_read = file.read(&mut header).ok()?;
    let buf = &header[..bytes_read];

    if buf.len() >= 4 && buf[0] == 0xFF && buf[1] == 0xD8 {
        let mut i = 2;
        while i + 4 <= buf.len() {
            if buf[i] != 0xFF {
                break;
            }
            let marker = buf[i + 1];
            if marker == 0xD9 || marker == 0xDA {
                break;
            }
            if i + 4 > buf.len() {
                break;
            }
            let len = u16::from_be_bytes([buf[i + 2], buf[i + 3]]) as usize;
            if marker == 0xE0 && i + 2 + len <= buf.len() && len >= 16 {
                let jfif = &buf[i + 4..i + 2 + len];
                if jfif.starts_with(b"JFIF\0") {
                    let units = jfif[7];
                    let x_dens = u16::from_be_bytes([jfif[8], jfif[9]]) as f32;
                    let y_dens = u16::from_be_bytes([jfif[10], jfif[11]]) as f32;
                    if x_dens > 0.0 && y_dens > 0.0 {
                        if units == 1 {
                            return Some(DPI::new(x_dens, y_dens));
                        } else if units == 2 {
                            return Some(DPI::new(x_dens * 2.54, y_dens * 2.54));
                        }
                    }
                }
            }
            i += 2 + len;
        }
    } else if buf.len() >= 8 && &buf[..8] == b"\x89PNG\r\n\x1a\n" {
        let mut i = 8;
        while i + 12 <= buf.len() {
            let length = u32::from_be_bytes([buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]) as usize;
            let chunk_type = &buf[i + 4..i + 8];
            if chunk_type == b"pHYs" && i + 8 + length <= buf.len() && length >= 9 {
                let phys = &buf[i + 8..i + 8 + length];
                let ppux = u32::from_be_bytes([phys[0], phys[1], phys[2], phys[3]]) as f32;
                let ppuy = u32::from_be_bytes([phys[4], phys[5], phys[6], phys[7]]) as f32;
                let unit = phys[8];
                if unit == 1 && ppux > 0.0 && ppuy > 0.0 {
                    return Some(DPI::new(ppux * 0.0254, ppuy * 0.0254));
                }
            }
            i += 12 + length;
        }
    }

    None
}

pub fn get_dpi(filename: &str, options: &Options) -> DPI {
    if let Some(user_dpi) = options.dpi {
        return DPI::uniform(user_dpi);
    }
    extract_dpi(filename).unwrap_or_default()
}

pub fn load_img(
    filename: &str,
    output_png: &str,
    options: &Options,
) -> Result<(Array3<u8>, DPI, Vec<Vec<u32>>, PdfPage), Box<dyn Error>> {
    let img = image::open(filename)
        .expect("Couldn't load the image!")
        .into_rgb8();
    let (width, height) = img.dimensions();
    let array: Array3<u8> =
        Array3::from_shape_vec((height as usize, width as usize, 3), img.into_raw()).unwrap();
    let dpi = get_dpi(filename, options);
    let sample_fraction = options.sample_fraction.parse().unwrap_or(5);
    let samples = sample_pixels(array.view(), sample_fraction);
    let palette = get_palette(&samples, options);
    let labels = apply_palette(array.view(), &palette, options);
    let labels_raw = labels.into_raw_vec_and_offset().0;
    let palette_saved = save(
        output_png,
        Array2::from_shape_vec((height as usize, width as usize), labels_raw.clone()).unwrap(),
        palette.clone(),
        options,
    );
    let adjusted_palette = adjust_palette(palette, options);
    let palette_u8: Vec<[u8; 3]> = adjusted_palette
        .iter()
        .map(|c| [c[0] as u8, c[1] as u8, c[2] as u8])
        .collect();

    let pdf_page = PdfPage {
        width,
        height,
        dpi,
        palette: palette_u8,
        labels: labels_raw,
    };

    Ok((array, dpi, palette_saved, pdf_page))
}
