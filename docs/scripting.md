```text
Scripts & presets use Rhai (Rust-like syntax: let, fn, if, for x in arr, for i in 0..n, #{ map }, [array]).
script_run executes code with:
  project — live handle:
    add_track(name[, comp]) → id; add_clip(track_or_"", #{clip}) → id; add_clip_in(comp, track, #{clip}); add_comp(id, w, h, fps, dur)
    add_effect(target_id, fx(...)) → id; update(id, #{merge patch}); set(id, "path.to.field", value); get(id, path); unset(id, path)
    keyframe(id, path, t, v[, ease]); expr(id, path, "expression" or ""); remove(id); split(id, t) → new id; move_clip(id, start[, track])
    duplicate(id, start) → id; add_marker(t, label); set_duration(s); set_var(name, v); set_project(path, v) (project-level: timing, meta, script_library...); apply_preset(target_or_"", preset, #{args})
    clip(id) / object(id) → map; tracks() → ids; clips(track) / clips() → ids; assets([kind]) → ids; asset(id); asset_duration(id); comp(); json(); new_id(prefix)
  timing: beats(), downbeats(), drops(), accents(), sections() (maps start/end/label/energy), bpm(), beats_between(a,b), nearest_beat(t)
  media: scenes(asset) → [#{start,end,motion,brightness,saturation,color,thumb}], subtitles(asset) → [#{start,end,text,words}]
  builders: fx(id[, params]), keys([[t,v,ease],...]), expr(code[, base]), hit_keys(at, base, peak, dur[, ease]),
            pulse_keys(times, base, peak, [attack,] release[, ease]), pick_times(ctx, "beats|downbeats|drops|half|bars2|accents", every), every(arr,n), dir_vec(dir, dist)
  map helpers (on clip maps): clip.add_fx(fx(..)), clip.set_kf(path, t, v, ease), clip.animate(path, keys), clip.set_path(path, v), clip.get_path(path), clip.merge(#{..})
  random (deterministic): rand(), rand(a,b), rand_int(a,b), choose(arr), shuffle(arr) · math: lerp, clamp, smoothstep, remap, hsv(h,s,v), ease_value(name,x) · log(x) / print(x)
  args (map) and ctx (fps, width, height, comp_duration, bpm, beat_len, beats/downbeats/drops/accents; clip presets also clip_id, clip_start, clip_duration and clip-local beats)
Clip presets additionally get `clip` (map) — edit it directly: clip.opacity = 0.5; clip.add_fx(fx("glow")); clip.transform = #{ scale: 1.2 }.
Preset file format: lines starting with //! form a TOML header:
//! id = "my_preset"
//! scope = "clip"          # or "timeline"
//! category = "impact"
//! description = "..."
//! params = [ { name = "at", type = "float", default = 0.0, desc = "..." } ]   # types: float int bool color enum(options=[..]) string vec2 point
<rhai code>
Example script: cut every 2 beats between two shots and punch on downbeats:
  let tr = project.add_track("cuts"); let bs = every(beats(), 2);
  for i in 0..bs.len()-1 { let c = project.add_clip(tr, #{start: bs[i], duration: bs[i+1]-bs[i], source: #{type:"media", asset: if i % 2 == 0 {"shotA"} else {"shotB"}}, fit:"cover"});
    project.apply_preset(c, "zoom_punch_beats", #{on: "downbeats"}); }

```
