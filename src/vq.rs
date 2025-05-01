use ndarray::{Array2, Axis};

pub fn vq(pixels_fg: &Array2<f32>, centroids_array: &Array2<u32>) -> Vec<u8> {
    let mut closest_centroids = Vec::with_capacity(pixels_fg.nrows());

    for p in pixels_fg.axis_iter(Axis(0)) {
        let mut min_d = f32::MAX;
        let mut closest_index = 0;

        for (i, centroid) in centroids_array.axis_iter(Axis(0)).enumerate() {
            let d = p
                .iter()
                .zip(centroid.iter())
                .map(|(p, c)| (p - *c as f32).powi(2))
                .sum::<f32>();

            if d < min_d {
                min_d = d;
                closest_index = i;
            }
        }
        closest_centroids.push(closest_index as u8);
    }
    closest_centroids
}
