```text
AMV craft tips for agents:
- Cut on beats; go faster (1 beat) in drops/choruses, slower (2-4 beats) in verses; match high-motion shots to high energy (detect_scenes motion).
- Hit the downbeats: impact / zoom_punch / flash_hit / shake_hit at the exact beat time (clip-local!). pulse_keys(times...) builds these.
- Velocity edits (velocity_ramp, velocity_beats) + motion_blur:true sell speed. Echo (clip.echo) gives afterimage trails.
- Use transition_in sparingly: flash_cut on drops, zoom_through / whip_pan for energy, crossfade/dreamy for calm parts.
- Put global looks on an adjustment clip (global_grade) instead of every clip. Overlays (sakura, speed_lines) go on top tracks with blend modes (screen/add).
- Text: lyrics_from_subtitles, text_* presets; stroke + shadow keeps text readable.
- Expressions keep things in sync automatically: scale {"expr":"value*(1.0+0.08*pulse(9.0))"}, rotation {"expr":"wiggle(2.0,1.5)"}.
- Always check: render_frames {count: 12} after big changes; render_frame at exact hit times; validate before export.
- Agents can create entirely new looks: effect_create for shaders, preset_create for reusable recipes, script_library for shared Rhai functions.

```
