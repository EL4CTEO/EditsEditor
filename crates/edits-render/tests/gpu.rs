use edits_core::{BlendMode, Color, Fit, MatteMode, TransformState, Vec2};
use edits_fx::{EffectKind, Library};
use edits_media::Frame;
use edits_render::{EffectCall, Globals, GpuContext, GpuOptions, MaskParams, OutputMode, Renderer, transform};

fn renderer() -> Option<Renderer> {
    match GpuContext::new(&GpuOptions::default()) {
        Ok(g) => Some(Renderer::new(g)),
        Err(e) => {
            eprintln!("no GPU: {e}; skipping");
            None
        }
    }
}

fn checker(w: u32, h: u32) -> Frame {
    let mut d = vec![];
    for y in 0..h {
        for x in 0..w {
            let on = ((x / 8) + (y / 8)) % 2 == 0;
            d.extend_from_slice(if on { &[255, 40, 120, 255] } else { &[20, 200, 255, 255] });
        }
    }
    Frame::new(w, h, d, false)
}

#[test]
fn place_composite_mask_readback() {
    let Some(mut r) = renderer() else { return };
    let src = checker(64, 32);
    let mut f = r.frame();
    let acc = f.target(128, 72);
    f.clear(acc, Color::BLACK.to_linear_premul());
    let tex = f.upload(Some(1), &src);
    let layer = f.target(128, 72);
    let st = TransformState { scale: Vec2::splat(0.5), rotation: 15.0, ..Default::default() };
    let pp = transform::place_params((64, 32), (128, 72), Fit::Contain, None, &st, (false, false), None, [1.0; 4]);
    f.place(layer, tex, std::slice::from_ref(&pp)).unwrap();
    // motion blur variant
    let mut st2 = st;
    st2.position = Vec2::new(10.0, 0.0);
    let pp2 = transform::place_params((64, 32), (128, 72), Fit::Contain, None, &st2, (false, false), None, [1.0; 4]);
    let layer2 = f.target(128, 72);
    f.place(layer2, tex, &[pp.clone(), pp2]).unwrap();
    let mask = f
        .mask(
            None,
            (128, 72),
            &MaskParams {
                shape: 1,
                mode: 0,
                invert: false,
                center: [0.0, 0.0],
                size: [80.0, 50.0],
                radius: 0.0,
                feather: 8.0,
                expansion: 0.0,
                opacity: 1.0,
                offset: [0.0, 0.0],
                rotation: 0.0,
                scale: [1.0, 1.0],
                points: vec![],
            },
        )
        .unwrap();
    let poly = f
        .mask(
            Some(mask),
            (128, 72),
            &MaskParams {
                shape: 2,
                mode: 1,
                invert: false,
                center: [0.0, 0.0],
                size: [0.0, 0.0],
                radius: 0.0,
                feather: 2.0,
                expansion: 0.0,
                opacity: 1.0,
                offset: [0.0, 0.0],
                rotation: 0.0,
                scale: [1.0, 1.0],
                points: vec![[-10.0, -10.0], [10.0, -10.0], [0.0, 10.0]],
            },
        )
        .unwrap();
    let mut acc = acc;
    for mode in BlendMode::ALL {
        acc = f.composite(acc, layer, *mode, 0.5, Some(poly), Some((layer2, MatteMode::Luma))).unwrap();
    }
    let out = f.read(acc, OutputMode::Straight).unwrap();
    assert_eq!((out.width, out.height), (128, 72));
    assert!(out.data.iter().any(|v| *v > 0));
    println!("stats: {:?}", r.stats);
}

#[test]
fn every_builtin_effect_runs() {
    let Some(mut r) = renderer() else { return };
    let lib = Library::builtin();
    let a = checker(96, 54);
    let mut ok = 0;
    for def in lib.effects() {
        let params: Vec<[f32; 4]> = def.slot_params().map(|p| p.to_slot(&p.default)).collect();
        let mut f = r.frame();
        let ta = f.upload(Some(10), &a);
        let tb = f.upload(Some(11), &checker(48, 48));
        let g = Globals {
            time: 1.3,
            local_time: 0.7,
            progress: 0.4,
            duration: 2.0,
            seed: 0.37,
            bpm: 120.0,
            beat_time: 0.1,
            ..Default::default()
        };
        let (input, input2) = match def.kind {
            EffectKind::Filter => (Some(ta), None),
            EffectKind::Transition => (Some(ta), Some(tb)),
            EffectKind::Generator => (None, None),
        };
        let out = f
            .effect(EffectCall { def, params: &params, globals: g, input, input2, extra: None, size: (96, 54) })
            .unwrap_or_else(|e| panic!("{}: {e}", def.id));
        let px = f.read(out, OutputMode::Straight).unwrap();
        assert_eq!(px.width, 96, "{}", def.id);
        ok += 1;
    }
    println!("{ok} effects rendered; stats {:?}", r.stats);
}
