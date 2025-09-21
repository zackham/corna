precision mediump float;
varying vec2 vUV;
varying vec2 vViewport;
uniform vec4 uColor;
uniform float uTime;
uniform int uEffectMode;
uniform float uProgress;

// Noise function for turbulence
float noise(vec2 p) {
  return sin(p.x * 10.0) * sin(p.y * 10.0);
}

// Fractal brownian motion for organic turbulence
float fbm(vec2 p, float t) {
  float value = 0.0;
  float amplitude = 0.5;
  float frequency = 1.0;

  for(int i = 0; i < 6; i++) {
    value += amplitude * sin(p.x * frequency + t) * cos(p.y * frequency - t * 0.7);
    p = p * 2.0 + vec2(sin(t * 0.1), cos(t * 0.13));
    frequency *= 2.1;
    amplitude *= 0.5;
  }

  return value;
}

// Vortex function
vec2 vortex(vec2 p, float t) {
  float angle = length(p) * 3.0 - t * 2.0;
  float radius = length(p);
  return vec2(
    cos(angle) * radius - sin(angle) * radius * 0.5,
    sin(angle) * radius + cos(angle) * radius * 0.5
  );
}

void main() {
  if (uEffectMode == 0) {
    gl_FragColor = uColor;
  } else if (uEffectMode == 1) {  // Reveal: mathematical wave interference
    vec2 p = vUV * 2.0 - 1.0;
    float t = uTime * 2.0;
    float angle = length(p) * 6.2832 + t;
    float wave = sin(angle * 8.0) * 0.5 + 0.5;
    wave *= sin(p.x * 20.0 + t * 1.5) * sin(p.y * 20.0 + t * 1.7);
    gl_FragColor = vec4(uColor.rgb, uColor.a * wave);
  } else if (uEffectMode == 2) {  // ULTRA RADICAL PLASMA CHAOS
    // Correct for aspect ratio to prevent distortion
    vec2 p = vUV * 2.0 - 1.0;
    float aspectRatio = vViewport.x / vViewport.y;
    p.x *= aspectRatio;

    // Use screen space coordinates for high-def visualization
    // This ensures patterns are sized correctly for the display
    vec2 screenCoord = vUV * vViewport / 100.0; // Normalize to reasonable units

    float t = uTime * 0.3;

    // Create multiple coordinate systems for chaos
    // Use both normalized and screen coordinates for variety
    vec2 p1 = p;
    vec2 p2 = vortex(p * 0.5, t);  // Scale down vortex for ultra-wide
    vec2 p3 = screenCoord * vec2(sin(t * 0.3), cos(t * 0.2));

    // Layer 1: Fractal turbulence - use screen coordinates for consistent detail
    float turb = fbm(screenCoord * 0.5, t);

    // Layer 2: Vortex dynamics with feedback
    vec2 vort = vortex(p2, t * 1.5);
    float vortPattern = sin(vort.x * 5.0 + turb * 3.0) * cos(vort.y * 5.0 - turb * 2.0);

    // Layer 3: Mathematical chaos - strange attractor inspired
    float chaos = 0.0;
    vec2 cp = p / aspectRatio;  // Normalize back for circular patterns
    for(int i = 0; i < 8; i++) {
      cp = vec2(
        sin(cp.y * 3.0 + t) * 1.5,
        cos(cp.x * 3.0 - t * 1.1) * 1.5
      );
      chaos += sin(length(cp) * 10.0 - float(i) * 0.5 + t * 2.0);
    }
    chaos /= 8.0;

    // Layer 4: Wave interference with multiple sources
    float waves = 0.0;
    for(int i = 0; i < 5; i++) {
      float fi = float(i);
      // Distribute sources across the wide screen
      vec2 source = vec2(
        sin(t * 0.7 + fi * 1.1) * aspectRatio * 0.8,
        cos(t * 0.5 + fi * 0.9) * 0.8
      );
      float dist = length(p - source);
      waves += sin(dist * 15.0 - t * 3.0 + fi) / (1.0 + dist);
    }

    // Layer 5: Reaction-diffusion inspired patterns
    // Use screen coordinates for consistent pattern density
    float reaction = sin(screenCoord.x * 3.0 + chaos * 5.0 + t) *
                    cos(screenCoord.y * 3.0 + vortPattern * 5.0 - t * 1.3);
    reaction = sin(reaction * 10.0 + waves);

    // Layer 6: Dynamic digital glitch effects with more movement
    vec2 glitchCoord = p + vec2(sin(t * 3.7) * 0.1, cos(t * 4.3) * 0.1);
    float glitch = sin(glitchCoord.y * (80.0 + sin(t * 2.0) * 40.0) + sin(glitchCoord.x * (60.0 + cos(t * 1.5) * 30.0) + t * 15.0)) *
                  step(0.96 + sin(t * 0.5) * 0.02, sin(t * 8.7 + glitchCoord.x * 45.0 + glitchCoord.y * 35.0));

    // Layer 7: Morphing fractal spirals
    float spiral = 0.0;
    vec2 sp = (p / aspectRatio) * (1.0 + sin(t * 0.25) * 0.3);  // Correct aspect for spirals
    for(int i = 0; i < 4; i++) {
      float angle = atan(sp.y, sp.x) + t * 0.3 * float(i + 1);
      float radius = length(sp) * (1.0 + sin(t * 0.4 + float(i)) * 0.2);
      spiral += sin(angle * (4.0 + float(i)) + radius * (8.0 + sin(t * 0.3) * 4.0) - t * (2.0 + float(i) * 0.5));
      sp = vec2(sp.x * sp.x - sp.y * sp.y, 2.0 * sp.x * sp.y) * (0.65 + sin(t * 0.2) * 0.1);
    }

    // Combine all layers with nonlinear mixing
    float plasma = turb * 2.0 + vortPattern + chaos * 1.5 + waves * 0.8 +
                  reaction * 0.5 + glitch * 0.3 + spiral * 0.4;

    // Add time-varying feedback
    plasma = sin(plasma * 2.0 + sin(plasma * 3.0 - t));

    // Create INSANE color dynamics
    vec3 col = vec3(0.0);

    // Red channel: Fast oscillations with chaos modulation
    col.r = sin(plasma * 3.14159 + t * 0.5 + chaos * 2.0) * 0.5 + 0.5;
    col.r = pow(col.r, 0.7 + sin(t * 0.3) * 0.2);

    // Green channel: Medium oscillations with vortex influence
    col.g = sin(plasma * 3.14159 + t * 0.7 + 2.094 + vortPattern) * 0.5 + 0.5;
    col.g = pow(col.g, 0.8 + cos(t * 0.2) * 0.15);

    // Blue channel: Slow oscillations with wave patterns
    col.b = sin(plasma * 3.14159 + t * 0.9 + 4.189 + waves * 0.5) * 0.5 + 0.5;
    col.b = pow(col.b, 0.9 + sin(t * 0.4) * 0.1);

    // Add chromatic aberration for psychedelic effect
    vec2 offset = vec2(sin(plasma * 5.0), cos(plasma * 5.0)) * 0.01;
    col.r = mix(col.r, sin(plasma * 4.0 + t) * 0.5 + 0.5, 0.3);
    col.b = mix(col.b, sin(plasma * 2.5 - t * 1.2) * 0.5 + 0.5, 0.3);

    // Add flowing iridescent highlights
    float highlight = pow(max(0.0, sin(plasma * (8.0 + sin(t * 0.6) * 4.0) + t * 1.5)), 2.5);
    vec3 iridescent = vec3(
      sin(highlight * (15.0 + sin(t * 0.3) * 10.0) + t * 2.5) * 0.5 + 0.5,
      sin(highlight * (15.0 + cos(t * 0.4) * 10.0) + t * 2.8 + 2.0) * 0.5 + 0.5,
      sin(highlight * (15.0 + sin(t * 0.5) * 10.0) + t * 3.2 + 4.0) * 0.5 + 0.5
    );
    col = mix(col, iridescent, highlight * (0.4 + sin(t * 0.7) * 0.2));

    // Add dynamic symmetry that constantly morphs
    // Use aspect-corrected coordinates for proper symmetry
    vec2 sym = (p / aspectRatio) * mat2(cos(t * 0.3), -sin(t * 0.3), sin(t * 0.3), cos(t * 0.3));
    float kaleid = sin(atan(sym.y, sym.x) * (5.0 + sin(t * 0.2)) + length(sym) * (8.0 + cos(t * 0.15) * 3.0) - t * 2.5);
    col = mix(col, vec3(kaleid * 0.5 + 0.5), 0.15 * (0.8 + sin(t * 0.4) * 0.2));

    // Boost saturation dramatically
    float gray = (col.r + col.g + col.b) / 3.0;
    col = mix(vec3(gray), col, 2.5 + sin(t * 0.1) * 0.5);

    // Add electric pulse based on plasma intensity instead of derivatives
    float intensity = abs(sin(plasma * 15.0));
    col += vec3(intensity * 0.1, intensity * 0.2, intensity * 0.4) * step(0.8, intensity);

    // Final color enhancement
    col = pow(col, vec3(0.8)); // Brighten
    col = clamp(col, 0.0, 1.0);

    // Add subtle pulse
    col *= 0.9 + sin(t * 4.0) * 0.1;

    // Fade in/out based on progress (0 to 1 over 5 seconds)
    // Fade in quickly (first 20%), full intensity (20-80%), fade out (last 20%)
    float alpha = 1.0;
    if (uProgress < 0.2) {
        alpha = uProgress * 5.0; // Quick fade in
    } else if (uProgress > 0.8) {
        alpha = (1.0 - uProgress) * 5.0; // Quick fade out
    }

    gl_FragColor = vec4(col, alpha);
  }
}