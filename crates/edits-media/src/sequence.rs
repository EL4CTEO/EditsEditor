//! Image sequences: `frames/img_%04d.png`, `frames/img_####.png`, or a directory of images.

use std::path::{Path, PathBuf};

use crate::{MediaError, Result, probe::IMAGE_EXT};

#[derive(Clone, Debug)]
pub struct ImageSequence {
    pub files: Vec<PathBuf>,
}

impl ImageSequence {
    pub fn open(pattern: &str) -> Result<ImageSequence> {
        let p = Path::new(pattern);
        let mut files = vec![];
        if p.is_dir() {
            for e in std::fs::read_dir(p)?.flatten() {
                let path = e.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                if IMAGE_EXT.contains(&ext.as_str()) {
                    files.push(path);
                }
            }
            files.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
        } else {
            // normalize #### to %04d
            let mut pat = pattern.to_string();
            if let Some(start) = pat.find('#') {
                let n = pat[start..].chars().take_while(|c| *c == '#').count();
                pat.replace_range(start..start + n, &format!("%0{n}d"));
            }
            let (pre, width, post) = parse_printf(&pat).ok_or_else(|| MediaError::Probe(format!("unsupported sequence pattern '{pattern}'")))?;
            let first = (0..=1).find(|i| Path::new(&format_index(&pre, width, *i, &post)).exists());
            let Some(mut i) = first else {
                return Err(MediaError::NotFound(pattern.to_string()));
            };
            loop {
                let f = PathBuf::from(format_index(&pre, width, i, &post));
                if !f.exists() {
                    break;
                }
                files.push(f);
                i += 1;
            }
        }
        if files.is_empty() {
            return Err(MediaError::NotFound(pattern.to_string()));
        }
        Ok(ImageSequence { files })
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn dimensions(&self) -> Result<(u32, u32)> {
        image::image_dimensions(&self.files[0]).map_err(|e| MediaError::Decode(e.to_string()))
    }
}

fn parse_printf(p: &str) -> Option<(String, usize, String)> {
    let i = p.find('%')?;
    let rest = &p[i + 1..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let after = &rest[digits.len()..];
    let after = after.strip_prefix('d')?;
    let width = digits.trim_start_matches('0').parse().unwrap_or(0);
    Some((p[..i].to_string(), width, after.to_string()))
}

fn format_index(pre: &str, width: usize, i: usize, post: &str) -> String {
    format!("{pre}{i:0width$}{post}")
}

/// Natural sort ("img2" < "img10").
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek(), bi.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, _) => return std::cmp::Ordering::Less,
            (_, None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(c) = ai.peek().filter(|c| c.is_ascii_digit()) {
                    na.push(*c);
                    ai.next();
                }
                let mut nb = String::new();
                while let Some(c) = bi.peek().filter(|c| c.is_ascii_digit()) {
                    nb.push(*c);
                    bi.next();
                }
                let o = na.parse::<u128>().unwrap_or(0).cmp(&nb.parse::<u128>().unwrap_or(0));
                if o != std::cmp::Ordering::Equal {
                    return o;
                }
            }
            (Some(x), Some(y)) => {
                let o = x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase());
                if o != std::cmp::Ordering::Equal {
                    return o;
                }
                ai.next();
                bi.next();
            }
        }
    }
}
