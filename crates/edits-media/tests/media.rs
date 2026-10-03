use std::path::Path;

use edits_media::{
    Ffmpeg, VideoOptions, VideoReader,
    encode::{Encoder, EncoderConfig, ExportSettings},
    probe::probe_asset,
    scenes::{SceneOptions, detect_scenes},
};

fn ff() -> Option<Ffmpeg> {
    Ffmpeg::locate().ok()
}

fn make_video(ff: &Ffmpeg, path: &Path) {
    // 2s red then 2s blue, 24fps, with a tone
    let st = ff
        .cmd()
        .args(["-y", "-f", "lavfi", "-i", "color=c=red:s=160x90:r=24:d=2", "-f", "lavfi", "-i", "color=c=blue:s=160x90:r=24:d=2"])
        .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=4"])
        .args(["-filter_complex", "[0:v][1:v]concat=n=2:v=1:a=0[v]", "-map", "[v]", "-map", "2:a", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest"])
        .arg(path)
        .status()
        .unwrap();
    assert!(st.success());
}

#[test]
fn decode_seek_scenes_encode() {
    let Some(ff) = ff() else {
        eprintln!("ffmpeg not available; skipping");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let v = dir.path().join("rb.mp4");
    make_video(&ff, &v);

    let (kind, info) = probe_asset(Some(&ff), &v, edits_core::AssetKind::Auto).unwrap();
    assert_eq!(kind, edits_core::AssetKind::Video);
    assert_eq!(info.width, 160);
    assert!(info.has_audio);
    assert!((info.fps.unwrap() - 24.0).abs() < 0.01);

    let mut r = VideoReader::open(&ff, &v, (160, 90), 24.0, info.duration.unwrap(), VideoOptions { size: None, fps: None, hwaccel: None }).unwrap();
    let f0 = r.frame_at(0.5).unwrap();
    assert!(f0.data[0] > 200 && f0.data[2] < 50, "expected red, got {:?}", &f0.data[..4]);
    let f1 = r.frame_at(3.0).unwrap();
    assert!(f1.data[2] > 200 && f1.data[0] < 50, "expected blue");
    // backward seek
    let f2 = r.frame_at(0.1).unwrap();
    assert!(f2.data[0] > 200);
    // sequential read-ahead should not seek
    let seeks = r.stats.seeks;
    for i in 3..20 {
        r.frame(i).unwrap();
    }
    assert_eq!(r.stats.seeks, seeks);

    let shots = detect_scenes(&ff, &v, 24.0, &SceneOptions::default()).unwrap();
    assert_eq!(shots.len(), 2, "{shots:?}");
    assert!((shots[1].start - 2.0).abs() < 0.1);

    let audio = edits_media::decode_audio(&ff, &v).unwrap();
    assert!((audio.duration() - 4.0).abs() < 0.2);
    let wav = dir.path().join("a.wav");
    audio.write_wav(&wav).unwrap();

    let out = dir.path().join("out.mp4");
    let settings = ExportSettings::new(out.to_string_lossy());
    let mut enc = Encoder::start(&ff, EncoderConfig { settings: &settings, width: 64, height: 32, fps: 24.0, audio_wav: Some(wav) }).unwrap();
    for i in 0..24u8 {
        enc.push(vec![i.wrapping_mul(10); 64 * 32 * 4]).unwrap();
    }
    let p = enc.finish().unwrap();
    let (k2, i2) = probe_asset(Some(&ff), &p, edits_core::AssetKind::Auto).unwrap();
    assert_eq!(k2, edits_core::AssetKind::Video);
    assert_eq!(i2.width, 64);
}
