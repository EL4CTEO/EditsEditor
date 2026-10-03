```text
EditsEditor: a GPU video editor you drive through tools — built for AMVs, anime edits, MVs and anything motion-graphics.

Typical workflow:
1. project_open {path, create:true, width, height, fps, duration}
2. media_import {paths:[folder or files]}  (video, audio, images, gif/webp/apng, svg, image sequences, fonts, .cube LUTs, .lrc/.srt lyrics)
3. Put the song on a track (clip_add), then analyze_audio {asset, apply:true} → beats, downbeats, drops, sections land in project timing.
4. detect_scenes / media_preview to look at your footage; pick shots (high motion for drops).
5. Build the edit: clip_add (cuts on beats), preset_apply (impacts, velocity, looks, text), effect_add, transitions (set transition_in),
   or write a script_run that loops over beats() and builds everything at once. preset_apply {preset:"auto_amv"} gives a full first pass.
6. LOOK at your work: render_frames {count:8} (contact sheet) and render_frame {time}. Iterate.
7. validate, then export {output:"out.mp4"}.

Key facts:
- Times are seconds. Keyframe times are clip-local (0 = clip start). Positions are pixels offset from the composition center (+y down).
- Any animatable property accepts: a value, {"keyframes":[[t,v,"ease"],...]}, and/or {"expr":"value * (1.0 + 0.1*pulse(8.0))"}.
- Expressions can read beats/audio: pulse(), bass(), beat_phase(), wiggle()... (reference topic "expressions").
- Effects are WGSL shaders (effects_list / effect_info). Write your own with effect_create. Presets are Rhai scripts (presets_list / preset_info); write your own with preset_create.
- Every mutation is undoable (history) and autosaved to the project file; the viewer app (edits view) live-updates.
- Use project_summary often — it is the cheapest way to see the whole timeline.
Read `reference` topics: workflow, format, expressions, scripting, effects, presets, easings, blend_modes, tips.

```
