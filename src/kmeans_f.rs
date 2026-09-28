use crate::arg::Options;

struct WeightedPoint {
    color: [f32; 3],
    weight: f32,
}

struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((self.0 >> 40) as u32) as f32 / 16777216.0
    }
}

fn dist_sq(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let dr = a[0] - b[0];
    let dg = a[1] - b[1];
    let db = a[2] - b[2];
    dr * dr + dg * dg + db * db
}

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

    // Quantize 24-bit RGB space into a 5-bit uniform 3D color cube (32x32x32 = 32,768 bins).
    // Shifting each channel by 3 (dividing by 8) maps [0, 255] into [0, 31].
    // Packing (br << 10) | (bg << 5) | bb creates a 15-bit linear index into counts and sums.
    // For each populated cell, we compute its exact barycenter (sums / count) and weight,
    // reducing thousands of raw sampled pixels down to ~500-1,500 unique weighted points.
    let mut counts = vec![0u32; 32768];
    let mut sums = vec![[0.0f32; 3]; 32768];

    for chunk in points.chunks_exact(3) {
        let r = (chunk[0] as f32).clamp(0.0, 255.0);
        let g = (chunk[1] as f32).clamp(0.0, 255.0);
        let b = (chunk[2] as f32).clamp(0.0, 255.0);

        let br = (r as usize >> 3).min(31);
        let bg = (g as usize >> 3).min(31);
        let bb = (b as usize >> 3).min(31);
        let idx = (br << 10) | (bg << 5) | bb;

        counts[idx] += 1;
        sums[idx][0] += r;
        sums[idx][1] += g;
        sums[idx][2] += b;
    }

    let mut weighted: Vec<WeightedPoint> = Vec::with_capacity(2048);
    for idx in 0..32768 {
        let count = counts[idx];
        if count > 0 {
            let inv = 1.0 / count as f32;
            weighted.push(WeightedPoint {
                color: [sums[idx][0] * inv, sums[idx][1] * inv, sums[idx][2] * inv],
                weight: count as f32,
            });
        }
    }

    if weighted.len() <= k {
        let mut res = Vec::with_capacity(weighted.len() + 1);
        res.push(vec![
            bg_color.0 as u32,
            bg_color.1 as u32,
            bg_color.2 as u32,
        ]);
        for w in weighted {
            res.push(vec![
                w.color[0].round().clamp(0.0, 255.0) as u32,
                w.color[1].round().clamp(0.0, 255.0) as u32,
                w.color[2].round().clamp(0.0, 255.0) as u32,
            ]);
        }
        return res;
    }

    let mut rng = SimpleRng::new(42);
    let mut centroids = Vec::with_capacity(k);
    let mut min_dists = vec![f32::MAX; weighted.len()];

    let first_idx = (rng.next_f32() * weighted.len() as f32) as usize % weighted.len();
    centroids.push(weighted[first_idx].color);

    for _ in 1..k {
        let last_c = centroids.last().unwrap();
        let mut total_weight = 0.0f32;

        for (i, p) in weighted.iter().enumerate() {
            let d = dist_sq(&p.color, last_c);
            if d < min_dists[i] {
                min_dists[i] = d;
            }
            total_weight += p.weight * min_dists[i];
        }

        if total_weight <= 0.0 {
            centroids.push(weighted[centroids.len() % weighted.len()].color);
            continue;
        }

        let threshold = rng.next_f32() * total_weight;
        let mut cum_weight = 0.0f32;
        let mut selected = 0;

        for (i, p) in weighted.iter().enumerate() {
            cum_weight += p.weight * min_dists[i];
            if cum_weight >= threshold {
                selected = i;
                break;
            }
        }

        centroids.push(weighted[selected].color);
    }

    let mut assignments = vec![0usize; weighted.len()];
    let mut uppers = vec![0.0f32; weighted.len()];
    let mut lowers = vec![f32::MAX; weighted.len()];

    let mut c_dist = vec![vec![0.0f32; k]; k];
    let mut s_dist = vec![0.0f32; k];

    for (i, p) in weighted.iter().enumerate() {
        let mut best_c = 0;
        let mut best_d = f32::MAX;
        let mut second_d = f32::MAX;

        for (c_idx, c) in centroids.iter().enumerate() {
            let d = dist_sq(&p.color, c).sqrt();
            if d < best_d {
                second_d = best_d;
                best_d = d;
                best_c = c_idx;
            } else if d < second_d {
                second_d = d;
            }
        }

        assignments[i] = best_c;
        uppers[i] = best_d;
        lowers[i] = second_d;
    }

    let max_iter = 40;
    let mut new_sums = vec![[0.0f32; 3]; k];
    let mut new_weights = vec![0.0f32; k];
    let mut shifts = vec![0.0f32; k];

    for _ in 0..max_iter {
        for a in 0..k {
            let mut min_c = f32::MAX;
            for b in 0..k {
                if a == b {
                    c_dist[a][b] = 0.0;
                } else {
                    let d = 0.5 * dist_sq(&centroids[a], &centroids[b]).sqrt();
                    c_dist[a][b] = d;
                    if d < min_c {
                        min_c = d;
                    }
                }
            }
            s_dist[a] = min_c;
        }

        for i in 0..k {
            new_sums[i] = [0.0; 3];
            new_weights[i] = 0.0;
        }

        for (i, p) in weighted.iter().enumerate() {
            let mut c = assignments[i];
            let mut u = uppers[i];

            if u > s_dist[c] {
                let mut best_d = dist_sq(&p.color, &centroids[c]).sqrt();
                u = best_d;
                let mut second_d = lowers[i];

                for j in 0..k {
                    if j == c {
                        continue;
                    }

                    if u > c_dist[c][j] && u > lowers[i] {
                        let d = dist_sq(&p.color, &centroids[j]).sqrt();
                        if d < best_d {
                            second_d = best_d;
                            best_d = d;
                            c = j;
                        } else if d < second_d {
                            second_d = d;
                        }
                    }
                }

                assignments[i] = c;
                uppers[i] = best_d;
                lowers[i] = second_d;
            }

            new_sums[c][0] += p.weight * p.color[0];
            new_sums[c][1] += p.weight * p.color[1];
            new_sums[c][2] += p.weight * p.color[2];
            new_weights[c] += p.weight;
        }

        let mut max_shift = 0.0f32;
        for c in 0..k {
            if new_weights[c] > 0.0 {
                let inv_w = 1.0 / new_weights[c];
                let new_c = [
                    new_sums[c][0] * inv_w,
                    new_sums[c][1] * inv_w,
                    new_sums[c][2] * inv_w,
                ];
                let shift = dist_sq(&centroids[c], &new_c).sqrt();
                shifts[c] = shift;
                if shift > max_shift {
                    max_shift = shift;
                }
                centroids[c] = new_c;
            } else {
                shifts[c] = 0.0;
            }
        }

        if max_shift < 0.1 {
            break;
        }

        for i in 0..weighted.len() {
            let c = assignments[i];
            uppers[i] += shifts[c];

            let mut max_other_shift = 0.0f32;
            for j in 0..k {
                if j != c && shifts[j] > max_other_shift {
                    max_other_shift = shifts[j];
                }
            }
            lowers[i] -= max_other_shift;
        }
    }

    let mut vivec = Vec::with_capacity(k + 1);
    vivec.push(vec![
        bg_color.0 as u32,
        bg_color.1 as u32,
        bg_color.2 as u32,
    ]);

    for c in centroids {
        vivec.push(vec![
            c[0].round().clamp(0.0, 255.0) as u32,
            c[1].round().clamp(0.0, 255.0) as u32,
            c[2].round().clamp(0.0, 255.0) as u32,
        ]);
    }

    vivec
}
