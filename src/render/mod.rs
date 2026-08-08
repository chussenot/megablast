//! wgpu pipeline setup: instanced quads, one pipeline, sRGB, <= 3 draw
//! calls per frame (background, entities, text). Every wgpu/winit type
//! in the crate lives under `src/render/`.
//!
//! Ported from arkanoid's `render.rs` (same wgpu 30.x / winit 0.30
//! pipeline: one shared unit-quad vertex buffer, one instance buffer of
//! `QuadInstance { center, half_size, color }`, one WGSL shader, sRGB
//! surface format; same glyphon text plumbing: `FontSystem`/`SwashCache`/
//! `TextAtlas`/`TextRenderer`/`Viewport`). `frame_events` drives Wave 5
//! `juice-fx`: muzzle flash and enemy-death bursts (`sprites::build`), HP
//! bar flash (`hud::build`), and screen shake (`Renderer::shake_timer`,
//! applied as a shared per-frame offset on every instance below).
//! Background + entities + text is exactly the module's <= 3 draw call
//! budget.

mod background;
mod hud;
mod sprites;

use std::future::Future;
use std::mem::size_of;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use glyphon::cosmic_text::Align;
use glyphon::{
    Attrs, Buffer as TextBuffer, Cache as TextCache, Family, FontSystem, Metrics, Resolution,
    Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::events::GameEvent;
use crate::game::{Game, PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};

/// One corner of the shared unit quad every entity is drawn from. Kept in
/// `[-1, 1]` on both axes so the vertex shader can place it with a single
/// multiply-add against an instance's `half_size`/`center` (same trick
/// arkanoid's `render.rs` uses).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    corner: [f32; 2],
}

/// Two triangles covering the unit quad. No index buffer: six vertices is
/// little enough data that an index buffer would only add a second buffer
/// to manage for no real savings.
const QUAD_VERTICES: [Vertex; 6] = [
    Vertex {
        corner: [-1.0, -1.0],
    },
    Vertex {
        corner: [1.0, -1.0],
    },
    Vertex { corner: [1.0, 1.0] },
    Vertex {
        corner: [-1.0, -1.0],
    },
    Vertex { corner: [1.0, 1.0] },
    Vertex {
        corner: [-1.0, 1.0],
    },
];

/// Per-instance data for one quad: where it is, how big it is, and its
/// color. Every drawable thing -- starfield dot, ship, and (in later
/// waves) enemies/shots/bullets/pickups/juice quads -- becomes one of
/// these; the pipeline itself never changes. Default (module-private)
/// visibility is enough for `background`/`sprites` to use it: they're
/// descendant modules of `render`, so they already see private items
/// defined here via `super::QuadInstance`.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct QuadInstance {
    center: [f32; 2],
    half_size: [f32; 2],
    color: [f32; 4],
}

impl QuadInstance {
    fn new(center: [f32; 2], half_size: [f32; 2], color: [f32; 4]) -> Self {
        Self {
            center,
            half_size,
            color,
        }
    }
}

/// Ceiling on total quad instances the instance buffer has room for: the
/// starfield layer plus headroom for entities (`sprites::MAX_ENTITY_QUADS`)
/// plus the HUD's own quads (e.g. an HP bar, `hud::MAX_HUD_QUADS`) -- bump
/// either constant in its own file if a future wave's worst-case frame
/// needs more room than it already reserves.
const MAX_QUADS: usize = background::STAR_COUNT + sprites::MAX_ENTITY_QUADS + hud::MAX_HUD_QUADS;

/// Approximation of one frame's wall-clock duration, used to decay the
/// screen-shake timer and enemy-death particles' life. `render()` has no
/// real `dt` (it's driven by redraw requests, not the fixed-timestep sim
/// loop) -- a fixed 60 Hz-ish decrement is the cheap option the spec
/// explicitly allows in place of tracking an `Instant` per frame, and
/// juice timing doesn't need to be exact.
const APPROX_FRAME_DT: f32 = 1.0 / 60.0;
/// Screen-shake duration on `GameEvent::BossHit` (spec: "screen shake 4
/// px/100 ms on boss hits").
const SHAKE_DURATION: f32 = 0.1;
/// Screen-shake amplitude in px -- applied as one shared random offset
/// to every instance's center per frame (spec: "cheapest: offset the
/// whole scene by one shared random (dx,dy) per frame, not
/// per-instance"), not a per-instance jitter.
const SHAKE_AMPLITUDE: f32 = 4.0;

// The shader below hardcodes the playfield size as WGSL consts (see its
// own comment for why). This guard catches the two numbers drifting
// apart at compile time instead of as a silently squished/stretched
// playfield if `game/mod.rs`'s constants ever change.
const _: () = assert!(PLAYFIELD_WIDTH as u32 == 600 && PLAYFIELD_HEIGHT as u32 == 800);

const SHADER_SRC: &str = r#"
struct VertexInput {
    @location(0) corner: vec2<f32>,
};

struct InstanceInput {
    @location(1) center: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// Fixed logical playfield (spec: 600x800 portrait, letterboxed --
// simulation space never scales on resize). Hardcoded rather than passed
// as a uniform: the value truly never changes, so a bind group would
// only add ceremony -- same call arkanoid made for its 800x600 field.
// This stretches the 600x800 field to fill whatever the surface size is;
// proper aspect-preserving letterboxing on resize is a later milestone,
// same limitation `Renderer::resize` already documents.
const PLAYFIELD_WIDTH: f32 = 600.0;
const PLAYFIELD_HEIGHT: f32 = 800.0;

@vertex
fn vs_main(vert: VertexInput, inst: InstanceInput) -> VertexOutput {
    let pixel_pos = inst.center + vert.corner * inst.half_size;
    let ndc_x = (pixel_pos.x / PLAYFIELD_WIDTH) * 2.0 - 1.0;
    let ndc_y = 1.0 - (pixel_pos.y / PLAYFIELD_HEIGHT) * 2.0;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.color = inst.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

const VERTEX_ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];
const INSTANCE_ATTRS: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![1 => Float32x2, 2 => Float32x2, 3 => Float32x4];

/// Snapshot of drawable state as of a tick boundary, so `Renderer::render`
/// can interpolate between this and the live `Game` using `alpha` (same
/// pattern as arkanoid's `RenderState`) -- avoids visible stutter when
/// the display's refresh rate isn't a multiple of the 120 Hz sim rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderState {
    player_x: f32,
    player_y: f32,
    scroll_y: f32,
}

impl From<&Game> for RenderState {
    fn from(game: &Game) -> Self {
        Self {
            player_x: game.player.x,
            player_y: game.player.y,
            scroll_y: game.scroll_y,
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Applies this frame's juice-relevant events to `Renderer`'s persistent
/// juice-fx state: a `BossHit` (re)arms the screen-shake timer, an
/// `EnemyDied` spawns a burst of shrinking fragments. Factored out of
/// `Renderer::render` as a free function so this branch is testable
/// without a wgpu device.
fn apply_juice_events(
    frame_events: &[GameEvent],
    shake_timer: &mut f32,
    death_particles: &mut Vec<sprites::DeathParticle>,
) {
    for event in frame_events {
        match event {
            GameEvent::BossHit { .. } => *shake_timer = SHAKE_DURATION,
            GameEvent::EnemyDied { x, y, .. } => {
                death_particles.extend(sprites::spawn_death_particles(*x, *y));
            }
            _ => {}
        }
    }
}

impl RenderState {
    /// Blends `prev` toward `curr` by `alpha` (0 = `prev`, 1 = `curr`),
    /// clamped so a caller passing a slightly-over-budget accumulator
    /// can't extrapolate past `curr`.
    fn lerp(prev: &Self, curr: &Self, alpha: f32) -> Self {
        let alpha = alpha.clamp(0.0, 1.0);
        Self {
            player_x: lerp(prev.player_x, curr.player_x, alpha),
            player_y: lerp(prev.player_y, curr.player_y, alpha),
            scroll_y: lerp(prev.scroll_y, curr.scroll_y, alpha),
        }
    }
}

/// Builds a single-line (or wrapped, if it's long enough) glyphon text
/// buffer, shaped and ready to hand to `TextRenderer::prepare` via a
/// `TextArea` -- same helper shape as arkanoid's `make_line_buffer`.
/// `wrap_width` also doubles as the box `align` centers/aligns within;
/// callers building HUD text (`hud::build`) pass the full surface width
/// for anything that should center across the screen.
pub(super) fn make_line_buffer(
    font_system: &mut FontSystem,
    text: &str,
    font_size: f32,
    line_height: f32,
    wrap_width: f32,
    align: Align,
) -> TextBuffer {
    let mut buffer = TextBuffer::new(font_system, Metrics::new(font_size, line_height));
    buffer.set_size(Some(wrap_width), None);
    buffer.set_text(
        text,
        &Attrs::new().family(Family::SansSerif),
        Shaping::Basic,
        Some(align),
    );
    buffer.shape_until_scroll(font_system, false);
    buffer
}

/// Builds one `TextArea` covering the whole surface horizontally at
/// `(left, top)`, uniformly colored `color` -- every HUD text element is a
/// single independently-positioned line/block, so they all share this one
/// shape (same as arkanoid's `text_area`).
fn text_area(buffer: &TextBuffer, left: f32, top: f32, color: glyphon::Color) -> TextArea<'_> {
    TextArea {
        buffer,
        left,
        top,
        scale: 1.0,
        bounds: TextBounds::default(),
        default_color: color,
        custom_glyphs: &[],
    }
}

/// Owns the wgpu instance-derived state for one window: the
/// adapter-negotiated device/queue, the surface configured to present to
/// that window, the one shared quad pipeline, and the starfield's fixed
/// star layout.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    quad_vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    starfield: background::Starfield,
    // -- text: HUD cash/score/level, shop screen (Wave 4), any future
    // overlay -- ported from arkanoid's identical setup.
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    // -- Wave 5 `juice-fx` persistent state: counts down after a
    // `GameEvent::BossHit`, while positive every instance this frame is
    // offset by one shared random (dx, dy) (see `render`'s doc comment).
    shake_timer: f32,
    // -- Wave 5 `juice-fx`: enemy-death burst fragments that outlive a
    // single frame, decayed and pruned every `render()` call -- see
    // `sprites::DeathParticle`.
    death_particles: Vec<sprites::DeathParticle>,
}

impl Renderer {
    /// Negotiates an adapter/device for `window` and configures its
    /// surface (sRGB format, vsync-on present mode). Blocks on
    /// adapter/device acquisition -- this only runs once at startup, so
    /// synchronous is simplest.
    pub fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window)
            .expect("failed to create wgpu surface");

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("failed to find a compatible wgpu adapter");

        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("megablast device"),
            ..Default::default()
        }))
        .expect("failed to request wgpu device");

        let caps = surface.get_capabilities(&adapter);
        // sRGB per spec; fall back to the adapter's top preference if it
        // somehow offers no sRGB format at all.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        // Vsync on: FifoRelaxed where supported (avoids a stutter when
        // the frame misses vsync by a hair), Fifo otherwise -- both are
        // guaranteed-supported-or-better vsync modes.
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::FifoRelaxed) {
            wgpu::PresentMode::FifoRelaxed
        } else {
            wgpu::PresentMode::Fifo
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quad shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quad pipeline layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("quad pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &VERTEX_ATTRS,
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: size_of::<QuadInstance>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &INSTANCE_ATTRS,
                    }),
                ],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    // Alpha blending, not `REPLACE`: every quad drawn so
                    // far has alpha 1.0, so blending produces the exact
                    // same pixels `REPLACE` would -- but it's what a
                    // future scrim/flash/trail quad (later waves) needs
                    // to actually blend instead of painting over the
                    // scene as a solid rectangle.
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad vertices"),
            contents: bytemuck::cast_slice(&QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        // Written fresh every frame via `queue.write_buffer` in
        // `render()`, so no initial contents -- just reserve room for
        // MAX_QUADS.
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad instances"),
            size: (size_of::<QuadInstance>() * MAX_QUADS) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // `text_cache` isn't kept on `Renderer`: `TextAtlas::new` clones it
        // internally (a cheap `Arc` underneath, shared pipeline/layout
        // state), and `Viewport` doesn't need it past construction --
        // same as arkanoid's identical setup.
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let text_cache = TextCache::new(&device);
        let viewport = Viewport::new(&device, &text_cache);
        let mut atlas = TextAtlas::new(&device, &queue, &text_cache, config.format);
        let text_renderer =
            TextRenderer::new(&mut atlas, &device, wgpu::MultisampleState::default(), None);

        Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            quad_vertex_buffer,
            instance_buffer,
            starfield: background::Starfield::new(),
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            shake_timer: 0.0,
            death_particles: Vec::new(),
        }
    }

    /// Reconfigures the surface after a window resize.
    ///
    /// A zero-area size (window minimized) is skipped: wgpu forbids
    /// configuring to zero, and there is nothing to render to anyway.
    /// Scaling/letterboxing the fixed 600x800 playfield into the new
    /// size is a later milestone -- this just keeps the surface valid so
    /// resizing doesn't crash.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Maps a point in the logical 600x800 playfield to the physical
    /// surface pixel space glyphon's `Viewport`/`TextArea` operate in,
    /// using the same non-uniform stretch the quad vertex shader already
    /// applies to every other quad (see `SHADER_SRC`) -- keeps HUD text
    /// aligned with the quads at whatever size the window's been resized
    /// to (same limitation `resize` documents: true aspect-preserving
    /// letterboxing is a later milestone).
    pub(super) fn to_physical(&self, x: f32, y: f32) -> (f32, f32) {
        (
            x / PLAYFIELD_WIDTH * self.config.width as f32,
            y / PLAYFIELD_HEIGHT * self.config.height as f32,
        )
    }

    /// Clears the surface to `clear_color`, then draws the starfield
    /// background, entities (ship/enemies/shots/HUD quads) and HUD text
    /// as instances of the shared quad pipeline plus one glyphon pass --
    /// exactly 3 draw calls (background, entities, text).
    ///
    /// `prev`/`current` are the render-relevant state one fixed tick
    /// apart and `alpha` is how far into that tick the current
    /// wall-clock frame falls (`accumulator / dt_fixed`, 0..=1) -- see
    /// `RenderState` for why interpolating between them is what keeps
    /// motion smooth when the display's refresh rate isn't a multiple of
    /// 120 Hz.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        clear_color: wgpu::Color,
        prev: &RenderState,
        current: &Game,
        alpha: f32,
        frame_events: &[GameEvent],
    ) {
        // Juice-fx state driven by this frame's events (Wave 5): a boss
        // hit (re)arms the shake timer, an enemy death spawns a burst of
        // shrinking fragments that outlive this one frame.
        apply_juice_events(
            frame_events,
            &mut self.shake_timer,
            &mut self.death_particles,
        );
        self.shake_timer = (self.shake_timer - APPROX_FRAME_DT).max(0.0);
        sprites::decay_death_particles(&mut self.death_particles, APPROX_FRAME_DT);

        let drawn = RenderState::lerp(prev, &RenderState::from(current), alpha);

        let hud_draw = hud::build(&mut self.font_system, current, frame_events);

        let mut instances = self.starfield.instances(drawn.scroll_y);
        let entity_start = instances.len() as u32;
        instances.extend(sprites::build(
            drawn.player_x,
            drawn.player_y,
            current,
            frame_events,
            &self.death_particles,
        ));
        instances.extend(hud_draw.quads);
        let entity_end = instances.len() as u32;

        // Screen shake (spec: "4 px/100 ms on boss hits"): one shared
        // random offset applied to every instance's center this frame,
        // not a per-instance jitter -- the cheapest option the spec
        // names, and cheap enough to just always run the loop (skipped
        // when the timer's expired since dx/dy are both 0.0 then).
        if self.shake_timer > 0.0 {
            let dx = rand::random_range(-SHAKE_AMPLITUDE..=SHAKE_AMPLITUDE);
            let dy = rand::random_range(-SHAKE_AMPLITUDE..=SHAKE_AMPLITUDE);
            for instance in &mut instances {
                instance.center[0] += dx;
                instance.center[1] += dy;
            }
        }

        self.queue
            .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));

        let text_areas: Vec<TextArea> = hud_draw
            .texts
            .iter()
            .map(|t| {
                let (x, y) = self.to_physical(t.x, t.y);
                text_area(&t.buffer, x, y, t.color)
            })
            .collect();
        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.config.width,
                height: self.config.height,
            },
        );
        self.text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .expect("glyphon text preparation failed");

        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            // Surface config no longer matches the window; reconfigure
            // and pick it up next frame instead of presenting a stale
            // frame.
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            // Transient: nothing to draw to right now, try again next
            // frame.
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => return,
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame encoder"),
            });
        {
            // Scoped: the render pass borrows `encoder` and must be
            // dropped (ending the pass) before `encoder.finish()` below.
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("quad pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            // Draw call 1: background (starfield).
            pass.draw(0..QUAD_VERTICES.len() as u32, 0..entity_start);
            // Draw call 2: entities (ship, enemies, shots, HUD quads).
            pass.draw(0..QUAD_VERTICES.len() as u32, entity_start..entity_end);
            // Draw call 3: HUD/overlay text.
            self.text_renderer
                .render(&self.atlas, &self.viewport, &mut pass)
                .expect("glyphon text render failed");
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(surface_texture);
        // Reclaims atlas space for glyphs that stopped being used this
        // frame -- cheap no-op most frames, matters once different HUD/
        // shop screens have all drawn text at some point in the session.
        self.atlas.trim();
    }
}

/// Minimal single-purpose executor for the one-shot startup futures wgpu
/// hands back (`request_adapter`/`request_device`). Native backends
/// resolve these without ever really suspending; pulling in a full async
/// runtime crate just to drive two startup calls isn't in this project's
/// dependency budget.
///
/// ponytail: parks/wakes the calling thread rather than running a real
/// reactor. Fine for a couple of startup awaits; revisit with a real
/// executor (or add one to the dependency budget) if async work lands on
/// a hot path later.
fn block_on<F: Future>(future: F) -> F::Output {
    struct ThreadWaker(std::thread::Thread);

    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }

    let mut future = std::pin::pin!(future);
    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(player_x: f32, player_y: f32, scroll_y: f32) -> RenderState {
        RenderState {
            player_x,
            player_y,
            scroll_y,
        }
    }

    #[test]
    fn lerp_at_zero_and_one_returns_the_endpoints_unchanged() {
        let prev = state(100.0, 200.0, 10.0);
        let curr = state(140.0, 180.0, 30.0);

        assert_eq!(RenderState::lerp(&prev, &curr, 0.0), prev);
        assert_eq!(RenderState::lerp(&prev, &curr, 1.0), curr);
    }

    #[test]
    fn lerp_at_half_is_the_midpoint() {
        let prev = state(100.0, 200.0, 10.0);
        let curr = state(140.0, 180.0, 30.0);

        let mid = RenderState::lerp(&prev, &curr, 0.5);

        assert!((mid.player_x - 120.0).abs() < 1e-4);
        assert!((mid.player_y - 190.0).abs() < 1e-4);
        assert!((mid.scroll_y - 20.0).abs() < 1e-4);
    }

    #[test]
    fn lerp_clamps_an_out_of_range_alpha() {
        let prev = state(100.0, 200.0, 10.0);
        let curr = state(140.0, 180.0, 30.0);

        assert_eq!(RenderState::lerp(&prev, &curr, -1.0), prev);
        assert_eq!(RenderState::lerp(&prev, &curr, 2.0), curr);
    }

    #[test]
    fn boss_hit_arms_the_shake_timer() {
        let mut shake_timer = 0.0;
        let mut death_particles = Vec::new();
        let events = [GameEvent::BossHit { x: 1.0, y: 2.0 }];

        apply_juice_events(&events, &mut shake_timer, &mut death_particles);

        assert_eq!(shake_timer, SHAKE_DURATION);
        assert!(death_particles.is_empty());
    }

    #[test]
    fn enemy_died_spawns_a_death_burst() {
        let mut shake_timer = 0.0;
        let mut death_particles = Vec::new();
        let events = [GameEvent::EnemyDied {
            x: 5.0,
            y: 6.0,
            kind: crate::levels::EnemyKind::Popcorn,
            credit_value: 5,
        }];

        apply_juice_events(&events, &mut shake_timer, &mut death_particles);

        assert_eq!(shake_timer, 0.0);
        assert_eq!(death_particles.len(), 4);
    }

    #[test]
    fn unrelated_events_leave_juice_state_untouched() {
        let mut shake_timer = 0.0;
        let mut death_particles = Vec::new();
        let events = [
            GameEvent::LevelCleared,
            GameEvent::PlayerHit { x: 0.0, y: 0.0 },
        ];

        apply_juice_events(&events, &mut shake_timer, &mut death_particles);

        assert_eq!(shake_timer, 0.0);
        assert!(death_particles.is_empty());
    }
}
