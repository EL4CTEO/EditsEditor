```text
Expressions are Rhai code evaluated every frame; the last value is the property value.
Variables: value (keyframed value; number or [x,y]/[r,g,b,a] array), t (clip-local s), time (comp s),
  duration, progress (t/duration), fps, frame, width, height, bpm.
Beat sync (needs project timing; run analyze_audio with apply=true):
  pulse(decay=8) -> 1 on each beat decaying exponentially; downbeat_pulse(d); drop_pulse(d); accent_pulse(d)
  pulse_every(n, decay); beat(); bar(); beat_in_bar() 0..3; beat_phase() 0..1; since_beat(); since_downbeat();
  since_drop(); to_next_beat(); beat_toggle() 0/1; beat_len(); beats_to_sec(b); in_drop(window_s)
Audio reactive (0..~1, from the timing source track): bass(), low_mid(), high_mid(), treble(), level(), onset(), bass_at(dt)
Noise & randomness: wiggle(freq, amp[, seed]); wiggle2(freq, amp) -> [x,y]; jitter(rate, amp); flicker(rate) 0..1;
  noise(x[, seed]) -1..1; fbm(x); random(seed) 0..1; random_range(a,b,seed); rand_clip() (stable per clip)
Time: step_time(fps) (choppy 'on twos'); loop_time(period); tween(t0, t1, v0, v1[, ease])
Math: lerp, mix, clamp, smoothstep, remap, fract, fmod, sign, deg, rad, sin/cos/abs/floor/sqrt/... (Rhai builtins), PI
Easing: ease(name, x) e.g. ease("ease_out_expo", progress)
Vectors/colors: arrays support component-wise math: value * 1.2, value + [10.0, 0.0], [a,b] * [c,d];
  vec2(x,y), vec3, rgba(r,g,b,a), hsv(h,s,v), add(a,b), mul(a,k), mix(a,b,t)
Project variables: var("name"[, default])
Examples:
  scale:    value * (1.0 + 0.15 * pulse(10.0))            // zoom punch on every beat
  rotation: wiggle(3.0, 4.0)                              // handheld sway
  opacity:  if beat_toggle() == 0.0 { 1.0 } else { 0.4 }  // strobe per beat
  position: add(value, wiggle2(8.0, 20.0 * bass()))       // bass-driven shake
  effect param 'amount' of rgb_split: 30.0 * pulse(12.0) + 5.0 * treble()

```
