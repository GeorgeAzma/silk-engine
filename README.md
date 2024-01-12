# 2D Rust Graphics Library
### **Render 1M+ 2D shapes, with 240+ FPS**

## Features
- Super Simple API
- Colored rendering
- Outline rendering
- Outline color
- Outline width
- Efficient textured rendering
- Antialiasing
- Position/Rotation/Scale
- Render any NGon (Triangle, Rectangle, Pentagon, ... , Circle)
- Incredibly efficient
- Text rendering

## Example Usage:
``` Rust
gfx.color = [255, 255, 255, 255];
gfx.stroke_color = [255, 0, 255, 255];
gfx.stroke_width = 0.3;
gfx.rotation = 3.14;
gfx.roundness = 0.2;
gfx.set_image(my_image);
gfx.tri(0.0, 0.0, 1.0, 1.0);
gfx.circle(0.0, 0.0, 1.0);

gfx.bold = 0.5;
gfx.text("Efficient Text Rendering", 0.0, 0.0, 1.0);
```

Using WGPU for rendering.

## Future Plan:
- Add blurred shape rendering (simple)
- Add shadows/glow (or do them using blurred shapes)
- Fix multiple line rendering transparency
  - Tried to use a depth comparison texture, but it didn't work, might be wgpu bug.
- Fix weird outlines on smaller scales for text
- Fix wrong antialiasing for straight edges
- Shader data compression
- Render text using same shader

## Renderer Technical Details
- 2D primitives/shapes are rendered using a single shader for efficiency
- Font SDF is generated in compute shader using bezier curves directly (in 10ms for 96 chars on GTX 1060)
