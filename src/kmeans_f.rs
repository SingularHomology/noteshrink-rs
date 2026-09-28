use crate::arg::Options;

const BINS: usize = 32 * 32 * 32;
const MAX_ITER: usize = 40;
const CONVERGENCE: f32 = 0.1;

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

#[inline(always)]
fn dist_sq(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let dr = a[0] - b[0];
    let dg = a[1] - b[1];
    let db = a[2] - b[2];
    dr * dr + dg * dg + db * db
}

#[inline]
fn parse_k(options: &Options) -> usize {
    options
        .num_colors
        .parse::<usize>()
        .unwrap_or(8)
        .saturating_sub(1)
}

#[inline(always)]
fn nearest_two(centroids: &[[f32; 3]], p: &[f32; 3]) -> (usize, f32, f32) {
    let mut best = 0usize;
    let mut best_d = f32::MAX;
    let mut second_d = f32::MAX;
    for (j, c) in centroids.iter().enumerate() {
        let d = dist_sq(p, c);
        if d < best_d {
            second_d = best_d;
            best_d = d;
            best = j;
        } else if d < second_d {
            second_d = d;
        }
    }
    (best, best_d.sqrt(), second_d.sqrt())
}

pub fn kmeans_precheck(points: &[f64], options: &Options) -> bool {
    let k = parse_k(options);
    points.is_empty() || k == 0 || points.len() < k * 3
}

pub fn apply_kmeans(
    points: &[f64],
    bg_color: &(u8, u8, u8),
    options: &Options,
    check: bool,
) -> Vec<Vec<u32>> {
    let bg = vec![bg_color.0 as u32, bg_color.1 as u32, bg_color.2 as u32];
    if check || points.is_empty() {
        return vec![bg];
    }

    let n = points.len() / 3;
    let k = parse_k(options).min(n);
    if k == 0 {
        return vec![bg];
    }

    // Quantize 24-bit RGB space into a 5-bit uniform 3D color cube (32x32x32 = 32,768 bins).
    // Shifting each channel by 3 (dividing by 8) maps [0, 255] into [0, 31].
    // Interleaving [sum_r, sum_g, sum_b, count] per bin keeps each entry in a single cache line.
    // For each populated cell, we compute its exact barycenter (sums / count) and weight,
    // reducing thousands of raw sampled pixels down to ~500-1,500 unique weighted points.
    let mut bins = vec![[0.0f64; 4]; BINS];

    for px in points.chunks_exact(3) {
        let r = px[0].clamp(0.0, 255.0);
        let g = px[1].clamp(0.0, 255.0);
        let b = px[2].clamp(0.0, 255.0);

        let idx = (((r as usize) >> 3) << 10 | ((g as usize) >> 3) << 5 | ((b as usize) >> 3))
            & (BINS - 1);

        let bin = &mut bins[idx];
        bin[0] += r;
        bin[1] += g;
        bin[2] += b;
        bin[3] += 1.0;
    }

    let mut weighted: Vec<WeightedPoint> = Vec::with_capacity(2048);
    for bin in &bins {
        let count = bin[3];
        if count > 0.0 {
            let inv = 1.0 / count;
            weighted.push(WeightedPoint {
                color: [
                    (bin[0] * inv) as f32,
                    (bin[1] * inv) as f32,
                    (bin[2] * inv) as f32,
                ],
                weight: count as f32,
            });
        }
    }
    drop(bins);

    let m = weighted.len();
    if m <= k {
        let mut res = Vec::with_capacity(m + 1);
        res.push(bg);
        for w in &weighted {
            res.push(vec![
                w.color[0].round().clamp(0.0, 255.0) as u32,
                w.color[1].round().clamp(0.0, 255.0) as u32,
                w.color[2].round().clamp(0.0, 255.0) as u32,
            ]);
        }
        return res;
    }

    let mut rng = SimpleRng::new(42);
    let mut centroids: Vec<[f32; 3]> = Vec::with_capacity(k);
    let mut min_d = vec![f32::MAX; m];
    let mut cum = vec![0.0f32; m];

    let first = ((rng.next_f32() * m as f32) as usize).min(m - 1);
    centroids.push(weighted[first].color);

    while centroids.len() < k {
        let last = centroids.last().unwrap();
        let mut total = 0.0f32;
        for (i, p) in weighted.iter().enumerate() {
            let d = dist_sq(&p.color, last);
            if d < min_d[i] {
                min_d[i] = d;
            }
            total += p.weight * min_d[i];
            cum[i] = total;
        }

        if total <= 0.0 {
            let idx = centroids.len() % m;
            centroids.push(weighted[idx].color);
            continue;
        }

        let threshold = rng.next_f32() * total;
        let mut sel = cum.partition_point(|&c| c < threshold).min(m - 1);
        while sel + 1 < m && min_d[sel] * weighted[sel].weight == 0.0 {
            sel += 1;
        }
        centroids.push(weighted[sel].color);
    }
    drop(min_d);
    drop(cum);

    let mut assign = vec![0usize; m];
    let mut upper = vec![0.0f32; m];
    let mut lower = vec![0.0f32; m];

    for (i, p) in weighted.iter().enumerate() {
        let (best, bd, sd) = nearest_two(&centroids, &p.color);
        assign[i] = best;
        upper[i] = bd;
        lower[i] = sd;
    }

    let mut s = vec![0.0f32; k];
    let mut new_sums = vec![[0.0f64; 3]; k];
    let mut new_w = vec![0.0f64; k];
    let mut shifts = vec![0.0f32; k];

    for _ in 0..MAX_ITER {
        s.fill(f32::MAX);
        for a in 0..k {
            let ca = &centroids[a];
            for b in (a + 1)..k {
                let d = dist_sq(ca, &centroids[b]);
                if d < s[a] {
                    s[a] = d;
                }
                if d < s[b] {
                    s[b] = d;
                }
            }
        }
        for v in s.iter_mut() {
            *v = 0.5 * v.sqrt();
        }

        new_sums.fill([0.0; 3]);
        new_w.fill(0.0);

        for (i, p) in weighted.iter().enumerate() {
            let mut a = assign[i];
            let bound = s[a].max(lower[i]);

            if upper[i] > bound {
                let exact = dist_sq(&p.color, &centroids[a]).sqrt();
                upper[i] = exact;

                if exact > bound {
                    let (best, bd, sd) = nearest_two(&centroids, &p.color);
                    a = best;
                    assign[i] = best;
                    upper[i] = bd;
                    lower[i] = sd;
                }
            }

            let w = p.weight as f64;
            new_sums[a][0] += w * p.color[0] as f64;
            new_sums[a][1] += w * p.color[1] as f64;
            new_sums[a][2] += w * p.color[2] as f64;
            new_w[a] += w;
        }

        let mut max1 = 0.0f32;
        let mut max2 = 0.0f32;
        let mut max1_idx = usize::MAX;

        for c in 0..k {
            let shift = if new_w[c] > 0.0 {
                let inv = 1.0 / new_w[c];
                let nc = [
                    (new_sums[c][0] * inv) as f32,
                    (new_sums[c][1] * inv) as f32,
                    (new_sums[c][2] * inv) as f32,
                ];
                let sh = dist_sq(&centroids[c], &nc).sqrt();
                centroids[c] = nc;
                sh
            } else {
                0.0
            };
            shifts[c] = shift;
            if shift > max1 {
                max2 = max1;
                max1 = shift;
                max1_idx = c;
            } else if shift > max2 {
                max2 = shift;
            }
        }

        if max1 < CONVERGENCE {
            break;
        }

        for i in 0..m {
            let a = assign[i];
            upper[i] += shifts[a];
            lower[i] -= if a == max1_idx { max2 } else { max1 };
        }
    }

    let mut out = Vec::with_capacity(k + 1);
    out.push(bg);
    for c in centroids {
        out.push(vec![
            c[0].round().clamp(0.0, 255.0) as u32,
            c[1].round().clamp(0.0, 255.0) as u32,
            c[2].round().clamp(0.0, 255.0) as u32,
        ]);
    }
    out
}
