use crate::types::DPI;
use miniz_oxide::deflate::compress_to_vec_zlib;
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref};
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Clone)]
pub struct PdfPage {
    pub width: u32,
    pub height: u32,
    pub dpi: DPI,
    pub palette: Vec<[u8; 3]>,
    pub labels: Vec<u8>,
}

pub fn export_pdf(
    pages: &[PdfPage],
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if pages.is_empty() {
        return Ok(());
    }

    let compressed_pages: Vec<Vec<u8>> = pages
        .par_iter()
        .map(|page| compress_to_vec_zlib(&page.labels, 6))
        .collect();

    let mut pdf = Pdf::new();
    let catalog_ref = Ref::new(1);
    let pages_root_ref = Ref::new(2);

    let mut next_ref = 3;
    let mut page_meta = Vec::with_capacity(pages.len());
    for _ in pages {
        let p_ref = Ref::new(next_ref);
        let c_ref = Ref::new(next_ref + 1);
        let i_ref = Ref::new(next_ref + 2);
        next_ref += 3;
        page_meta.push((p_ref, c_ref, i_ref));
    }

    pdf.catalog(catalog_ref).pages(pages_root_ref);
    pdf.pages(pages_root_ref)
        .count(pages.len() as i32)
        .kids(page_meta.iter().map(|(p, _, _)| *p));

    for (i, page) in pages.iter().enumerate() {
        let (p_ref, c_ref, i_ref) = page_meta[i];
        let img_name = Name(b"Im0");

        let dpi_x = if page.dpi.x > 0.0 { page.dpi.x } else { 300.0 };
        let dpi_y = if page.dpi.y > 0.0 { page.dpi.y } else { 300.0 };
        let w_pt = (page.width as f32 * 72.0) / dpi_x;
        let h_pt = (page.height as f32 * 72.0) / dpi_y;

        let mut page_obj = pdf.page(p_ref);
        page_obj.parent(pages_root_ref);
        page_obj.media_box(Rect::new(0.0, 0.0, w_pt, h_pt));
        page_obj.contents(c_ref);
        page_obj.resources().x_objects().pair(img_name, i_ref);
        page_obj.finish();

        let mut content = Content::new();
        content.save_state();
        content.transform([w_pt, 0.0, 0.0, h_pt, 0.0, 0.0]);
        content.x_object(img_name);
        content.restore_state();
        pdf.stream(c_ref, &content.finish());

        let flat_palette: Vec<u8> = page.palette.iter().flat_map(|c| *c).collect();
        let mut img_obj = pdf.image_xobject(i_ref, &compressed_pages[i]);
        img_obj.width(page.width as i32);
        img_obj.height(page.height as i32);
        img_obj.bits_per_component(8);
        img_obj.filter(Filter::FlateDecode);

        let color_space = img_obj.color_space();
        color_space.indexed(
            Name(b"DeviceRGB"),
            (page.palette.len() - 1) as i32,
            &flat_palette,
        );

        img_obj.finish();
    }

    let mut file = File::create(output_path)?;
    file.write_all(&pdf.finish())?;
    Ok(())
}
