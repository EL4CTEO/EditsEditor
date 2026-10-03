//! `.cube` 3D LUT parsing. The cube is unwrapped into a 2D strip of `size*size x size` texels
//! (blue slices laid out horizontally) so it can be bound as a regular 2D texture.

#[derive(Clone, Debug)]
pub struct Lut3d {
    pub size: u32,
    /// RGBA f32, `size*size` wide, `size` tall.
    pub data: Vec<f32>,
}

impl Lut3d {
    pub fn width(&self) -> u32 {
        self.size * self.size
    }
    pub fn height(&self) -> u32 {
        self.size
    }

    pub fn parse_cube(text: &str) -> Result<Lut3d, String> {
        let mut size = 0u32;
        let mut min = [0.0f32; 3];
        let mut max = [1.0f32; 3];
        let mut values: Vec<[f32; 3]> = Vec::new();
        for line in text.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with('#') {
                continue;
            }
            let mut it = l.split_whitespace();
            let first = it.next().unwrap_or("");
            match first {
                "TITLE" => {}
                "LUT_3D_SIZE" => size = it.next().and_then(|s| s.parse().ok()).ok_or("bad LUT_3D_SIZE")?,
                "LUT_1D_SIZE" => return Err("1D LUTs are not supported".into()),
                "DOMAIN_MIN" => {
                    for m in &mut min {
                        *m = it.next().and_then(|s| s.parse().ok()).ok_or("bad DOMAIN_MIN")?;
                    }
                }
                "DOMAIN_MAX" => {
                    for m in &mut max {
                        *m = it.next().and_then(|s| s.parse().ok()).ok_or("bad DOMAIN_MAX")?;
                    }
                }
                _ => {
                    let r: f32 = first.parse().map_err(|_| format!("unexpected line '{l}'"))?;
                    let g: f32 = it.next().and_then(|s| s.parse().ok()).ok_or("bad LUT row")?;
                    let b: f32 = it.next().and_then(|s| s.parse().ok()).ok_or("bad LUT row")?;
                    values.push([r, g, b]);
                }
            }
        }
        if !(2..=128).contains(&size) {
            return Err(format!("LUT size {size} unsupported"));
        }
        let n = size as usize;
        if values.len() != n * n * n {
            return Err(format!("expected {} rows, got {}", n * n * n, values.len()));
        }
        let w = n * n;
        let mut data = vec![0.0f32; w * n * 4];
        // .cube order: r fastest, then g, then b
        for b in 0..n {
            for g in 0..n {
                for r in 0..n {
                    let v = values[r + g * n + b * n * n];
                    let x = r + b * n;
                    let y = g;
                    let o = (y * w + x) * 4;
                    for c in 0..3 {
                        data[o + c] = (v[c] - min[c]) / (max[c] - min[c]).max(1e-6);
                    }
                    data[o + 3] = 1.0;
                }
            }
        }
        Ok(Lut3d { size, data })
    }

    /// Identity LUT (useful as a default).
    pub fn identity(size: u32) -> Lut3d {
        let n = size as usize;
        let mut s = format!("LUT_3D_SIZE {size}\n");
        for b in 0..n {
            for g in 0..n {
                for r in 0..n {
                    let f = |x: usize| x as f32 / (n - 1) as f32;
                    s.push_str(&format!("{} {} {}\n", f(r), f(g), f(b)));
                }
            }
        }
        Lut3d::parse_cube(&s).unwrap()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn identity_roundtrip() {
        let l = super::Lut3d::identity(4);
        assert_eq!(l.width(), 16);
        assert_eq!(l.height(), 4);
        // texel r=3,g=0,b=0 -> (1,0,0)
        let o = 3 * 4;
        assert!((l.data[o] - 1.0).abs() < 1e-6);
    }
}
