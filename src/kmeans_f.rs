use kmeans::KMeans;
use kmeans::*;

pub fn kmeans_precheck(points: &[f64], options: &crate::arg::Options) -> bool {
    let k = options.num_colors.parse::<usize>().unwrap() - 1;
    k >= points.len()
}

pub fn apply_kmeans(
    points: &[f64],
    bg_color: &(u8, u8, u8),
    options: &crate::arg::Options,
    check: bool,
) -> Vec<Vec<u32>> {
    if check {
        vec![vec![
            bg_color.0 as u32,
            bg_color.1 as u32,
            bg_color.2 as u32,
        ]]
    } else {
        let (sample_cnt, sample_dims) = (&points.len() / 3, 3);

        let k = options.num_colors.parse::<usize>().unwrap() - 1;
        let kmeans_iter = 40;

        let kmean: KMeans<_, 8, _> = KMeans::new(
            points.to_owned(),
            sample_cnt,
            sample_dims,
            EuclideanDistance,
        );
        let result = kmean.kmeans_lloyd(
            k,
            kmeans_iter,
            KMeans::init_kmeanplusplus,
            &KMeansConfig::default(),
        );
        let mut vivec = Vec::new();
        for i in result.centroids.iter() {
            vivec.push(Vec::from(i));
        }

        vivec.insert(
            0,
            vec![bg_color.0 as f64, bg_color.1 as f64, bg_color.2 as f64],
        );

        vivec
            .into_iter()
            .map(|x| x.into_iter().map(|y| y as u32).collect::<Vec<u32>>())
            .collect::<Vec<Vec<u32>>>()
    }
}
