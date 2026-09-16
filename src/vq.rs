use rayon::prelude::*;

pub fn vq(pixels_fg: &[[f32; 3]], centroids: &[[f32; 3]]) -> Vec<u8> {
    pixels_fg
        .par_iter()
        .map(|p| {
            let mut min_d = f32::MAX;
            let mut closest_index = 0;

            for (i, c) in centroids.iter().enumerate() {
                let dr = p[0] - c[0];
                let dg = p[1] - c[1];
                let db = p[2] - c[2];
                let d = dr * dr + dg * dg + db * db;

                if d < min_d {
                    min_d = d;
                    closest_index = i;
                }
            }
            closest_index as u8
        })
        .collect()
}
