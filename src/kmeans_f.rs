use crate::arg::Options;
use kmeans_colors::get_kmeans;
use palette::Srgb;

pub fn kmeans_precheck(points: &[f64], options: &Options) -> bool {
    let k = options
        .num_colors
        .parse::<usize>()
        .unwrap_or(8)
        .saturating_sub(1);
    points.is_empty() || k == 0 || points.len() < k * 3
}

pub fn apply_kmeans(
    points: &[f64],
    bg_color: &(u8, u8, u8),
    options: &Options,
    check: bool,
) -> Vec<Vec<u32>> {
    if check || points.is_empty() {
        return vec![vec![
            bg_color.0 as u32,
            bg_color.1 as u32,
            bg_color.2 as u32,
        ]];
    }

    let n = points.len() / 3;
    let k = (options.num_colors.parse::<usize>().unwrap_or(8) - 1).min(n);
    if k == 0 {
        return vec![vec![
            bg_color.0 as u32,
            bg_color.1 as u32,
            bg_color.2 as u32,
        ]];
    }

    let rgb_points: Vec<Srgb<f32>> = points
        .chunks_exact(3)
        .map(|c| {
            Srgb::new(
                (c[0] / 255.0) as f32,
                (c[1] / 255.0) as f32,
                (c[2] / 255.0) as f32,
            )
        })
        .collect();

    let max_iter = 40;
    let converge = 0.0005;
    let res = get_kmeans(k, max_iter, converge, false, &rgb_points, 42);

    let mut vivec = Vec::with_capacity(k + 1);
    vivec.push(vec![
        bg_color.0 as u32,
        bg_color.1 as u32,
        bg_color.2 as u32,
    ]);

    for c in res.centroids {
        vivec.push(vec![
            (c.red * 255.0).round().clamp(0.0, 255.0) as u32,
            (c.green * 255.0).round().clamp(0.0, 255.0) as u32,
            (c.blue * 255.0).round().clamp(0.0, 255.0) as u32,
        ]);
    }

    vivec
}
