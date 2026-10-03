//! Lyrics / subtitle parsing: LRC (incl. enhanced word timing), SRT, WebVTT and ASS/SSA.

use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Cue {
    pub start: f64,
    pub end: f64,
    pub text: String,
    /// Per-word timings when available (enhanced LRC `<mm:ss.xx>` or ASS `\k`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<(f64, String)>,
}

fn parse_clock(s: &str) -> Option<f64> {
    // [hh:]mm:ss[.,]fff
    let s = s.trim().replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    let (h, m, sec) = match parts.as_slice() {
        [m, s] => (0.0, m.parse::<f64>().ok()?, s.parse::<f64>().ok()?),
        [h, m, s] => (h.parse::<f64>().ok()?, m.parse::<f64>().ok()?, s.parse::<f64>().ok()?),
        _ => return None,
    };
    Some(h * 3600.0 + m * 60.0 + sec)
}

pub fn parse(text: &str, ext: &str) -> Vec<Cue> {
    let text = text.trim_start_matches('\u{feff}');
    let mut cues = match ext.to_ascii_lowercase().as_str() {
        "lrc" => parse_lrc(text),
        "srt" | "vtt" => parse_srt(text),
        "ass" | "ssa" => parse_ass(text),
        _ => {
            if text.contains("-->") {
                parse_srt(text)
            } else if text.contains("Dialogue:") {
                parse_ass(text)
            } else {
                parse_lrc(text)
            }
        }
    };
    cues.retain(|c| !c.text.trim().is_empty());
    cues.sort_by(|a, b| a.start.total_cmp(&b.start));
    cues
}

fn parse_lrc(text: &str) -> Vec<Cue> {
    let mut offset = 0.0;
    let mut raw: Vec<(f64, String)> = vec![];
    for line in text.lines() {
        let mut rest = line.trim();
        let mut times = vec![];
        while let Some(r) = rest.strip_prefix('[') {
            let Some(end) = r.find(']') else { break };
            let tag = &r[..end];
            if let Some(o) = tag.strip_prefix("offset:") {
                offset = o.trim().parse::<f64>().unwrap_or(0.0) / 1000.0;
            } else if let Some(t) = parse_clock(tag) {
                times.push(t);
            }
            rest = &r[end + 1..];
        }
        for t in times {
            raw.push((t, rest.to_string()));
        }
    }
    raw.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut cues = vec![];
    for (i, (t, line)) in raw.iter().enumerate() {
        let end = raw.get(i + 1).map(|n| n.0).unwrap_or(t + 4.0);
        // enhanced LRC words: <00:12.34>word
        let mut words = vec![];
        let mut plain = String::new();
        let mut s = line.as_str();
        while let Some(i0) = s.find('<') {
            plain.push_str(&s[..i0]);
            let r = &s[i0 + 1..];
            if let Some(j) = r.find('>') {
                if let Some(wt) = parse_clock(&r[..j]) {
                    let after = &r[j + 1..];
                    let next = after.find('<').unwrap_or(after.len());
                    let w = after[..next].to_string();
                    if !w.trim().is_empty() {
                        words.push((wt - offset, w.trim().to_string()));
                    }
                }
                s = &r[j + 1..];
            } else {
                break;
            }
        }
        plain.push_str(s);
        let plain = if words.is_empty() { plain } else { words.iter().map(|w| w.1.clone()).collect::<Vec<_>>().join(" ") };
        cues.push(Cue { start: t - offset, end: end - offset, text: plain.trim().to_string(), words });
    }
    cues
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '<' | '{' => depth += 1,
            '>' | '}' => depth = (depth - 1).max(0),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

fn parse_srt(text: &str) -> Vec<Cue> {
    let mut cues = vec![];
    let norm = text.replace("\r\n", "\n");
    for block in norm.split("\n\n") {
        let lines: Vec<&str> = block.lines().collect();
        let Some(ti) = lines.iter().position(|l| l.contains("-->")) else { continue };
        let (a, b) = lines[ti].split_once("-->").unwrap();
        let b = b.split_whitespace().next().unwrap_or("");
        let (Some(s), Some(e)) = (parse_clock(a), parse_clock(b)) else { continue };
        let txt = lines[ti + 1..].join("\n");
        cues.push(Cue { start: s, end: e, text: strip_tags(&txt).trim().to_string(), words: vec![] });
    }
    cues
}

fn parse_ass(text: &str) -> Vec<Cue> {
    let mut cues = vec![];
    let mut fields: Vec<String> = vec![];
    for line in text.lines() {
        let l = line.trim();
        if let Some(f) = l.strip_prefix("Format:") {
            fields = f.split(',').map(|x| x.trim().to_ascii_lowercase()).collect();
        } else if let Some(d) = l.strip_prefix("Dialogue:") {
            let n = fields.len().max(10);
            let parts: Vec<&str> = d.splitn(n, ',').collect();
            let idx = |name: &str, def: usize| fields.iter().position(|f| f == name).unwrap_or(def);
            let (si, ei, ti) = (idx("start", 1), idx("end", 2), idx("text", n - 1));
            let (Some(s), Some(e)) = (parts.get(si).and_then(|x| parse_clock(x)), parts.get(ei).and_then(|x| parse_clock(x))) else { continue };
            let raw = parts.get(ti).copied().unwrap_or("");
            // karaoke \k timings (centiseconds)
            let mut words = vec![];
            let mut t = s;
            let mut rest = raw;
            while let Some(i0) = rest.find("{\\k") {
                let r = &rest[i0 + 3..];
                let r = r.trim_start_matches(['f', 'o', 'K']);
                let Some(j) = r.find('}') else { break };
                let cs: f64 = r[..j].parse().unwrap_or(0.0);
                let after = &r[j + 1..];
                let next = after.find('{').unwrap_or(after.len());
                let w = after[..next].trim().to_string();
                if !w.is_empty() {
                    words.push((t, w));
                }
                t += cs / 100.0;
                rest = &after[next..];
            }
            let txt = strip_tags(raw).replace("\\N", "\n").replace("\\n", "\n");
            cues.push(Cue { start: s, end: e, text: txt.trim().to_string(), words });
        }
    }
    cues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lrc() {
        let c = parse("[ti:x]\n[00:01.00]Hello\n[00:03.50]World <00:04.00>a <00:04.50>b\n", "lrc");
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].start, 1.0);
        assert_eq!(c[0].end, 3.5);
        assert_eq!(c[1].words.len(), 2);
    }

    #[test]
    fn srt() {
        let c = parse("1\n00:00:01,000 --> 00:00:02,500\n<i>Hi</i> there\n\n2\n00:00:03,000 --> 00:00:04,000\nBye\n", "srt");
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].text, "Hi there");
        assert_eq!(c[0].end, 2.5);
    }

    #[test]
    fn ass() {
        let t = "[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,{\\k50}Hel{\\k50}lo\n";
        let c = parse(t, "ass");
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].words.len(), 2);
    }
}
