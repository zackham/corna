attribute vec2 aPos;
attribute vec2 aUV;
uniform vec2 uViewport;      // in pixels
varying vec2 vUV;
varying vec2 vViewport;
void main() {
  vec2 ndc = (aPos / uViewport) * 2.0 - 1.0;
  gl_Position = vec4(ndc.x, -ndc.y, 0.0, 1.0);
  vUV = aUV;
  vViewport = uViewport;
}