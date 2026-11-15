//! Spike backend (feature `spike-vello-gpu`): draws a [`SceneDesc`] with vello (GPU compute via
//! wgpu) into an offscreen texture and reads it back. Native/headless only in R0.
//! Comparison-only; the reference renderer is [`crate::cpu`].

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};

use vello::kurbo::{Affine, BezPath, Circle, Rect, Stroke};
use vello::peniko::{Blob, Color, Fill, FontData};
use vello::wgpu;
pub use vello::AaConfig;
use vello::{Glyph, RenderParams, Renderer, RendererOptions};

use crate::scene::{Prim, Rgba};
use crate::RenderError;
use crate::SceneDesc;

fn err(e: impl std::fmt::Display) -> RenderError {
    RenderError::new(format!("vello/wgpu: {e}"))
}

fn color(c: Rgba) -> Color {
    Color::from_rgba8(c[0], c[1], c[2], c[3])
}

/// Headless GPU renderer: one wgpu device + one vello `Renderer`, reused across frames.
pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
    font: FontData,
    /// Adapter description (backend, name), for the report.
    pub adapter_info: String,
}

impl GpuRenderer {
    /// Creates the device and compiles vello's pipelines (all AA modes).
    ///
    /// # Errors
    /// No suitable GPU adapter, or device/pipeline creation failure.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Result<Self, RenderError> {
        pollster::block_on(Self::new_async())
    }

    /// Async constructor (the only one available on wasm32/WebGPU).
    ///
    /// # Errors
    /// No suitable GPU adapter, or device/pipeline creation failure.
    pub async fn new_async() -> Result<Self, RenderError> {
        let mut ctx = vello::util::RenderContext::new();
        let id = ctx
            .device(None)
            .await
            .ok_or_else(|| RenderError::new("no wgpu adapter/device"))?;
        let handle = ctx.devices.swap_remove(id);
        #[cfg(not(target_arch = "wasm32"))]
        let adapter_info = ctx
            .instance
            .enumerate_adapters(wgpu::Backends::all())
            .await
            .first()
            .map(|a| {
                let i = a.get_info();
                format!("{:?} {}", i.backend, i.name)
            })
            .unwrap_or_default();
        #[cfg(target_arch = "wasm32")]
        let adapter_info = String::from("WebGPU (browser)");
        let renderer = Renderer::new(&handle.device, RendererOptions::default()).map_err(err)?;
        let font = FontData::new(Blob::new(Arc::new(crate::cpu::FONT_BYTES.to_vec())), 0);
        Ok(Self {
            device: handle.device,
            queue: handle.queue,
            renderer,
            font,
            adapter_info,
        })
    }

    fn encode(&self, scene: &SceneDesc) -> Result<vello::Scene, RenderError> {
        let mut vs = vello::Scene::new();
        let id = Affine::IDENTITY;
        for p in scene.prims() {
            match p {
                Prim::Rect {
                    x,
                    y,
                    w,
                    h,
                    color: c,
                } => {
                    let (x, y) = (f64::from(*x), f64::from(*y));
                    let r = Rect::new(x, y, x + f64::from(*w), y + f64::from(*h));
                    vs.fill(Fill::NonZero, id, color(*c), None, &r);
                }
                Prim::Polyline {
                    points,
                    width,
                    color: c,
                } => {
                    let mut bp = BezPath::new();
                    for (i, &[x, y]) in points.iter().enumerate() {
                        let pt = (f64::from(x), f64::from(y));
                        if i == 0 {
                            bp.move_to(pt);
                        } else {
                            bp.line_to(pt);
                        }
                    }
                    vs.stroke(&Stroke::new(f64::from(*width)), id, color(*c), None, &bp);
                }
                Prim::Fill { points, color: c } => {
                    let mut bp = BezPath::new();
                    for (i, &[x, y]) in points.iter().enumerate() {
                        let pt = (f64::from(x), f64::from(y));
                        if i == 0 {
                            bp.move_to(pt);
                        } else {
                            bp.line_to(pt);
                        }
                    }
                    bp.close_path();
                    vs.fill(Fill::NonZero, id, color(*c), None, &bp);
                }
                Prim::Circle {
                    cx,
                    cy,
                    r,
                    color: c,
                } => {
                    let circle = Circle::new((f64::from(*cx), f64::from(*cy)), f64::from(*r));
                    vs.fill(Fill::NonZero, id, color(*c), None, &circle);
                }
                Prim::Text {
                    x,
                    baseline,
                    px,
                    text,
                    color: c,
                    align,
                } => {
                    let glyphs: Vec<Glyph> = crate::scene::layout_glyphs(text, *px, *x, *align)?
                        .into_iter()
                        .map(|(gid, gx)| Glyph {
                            id: gid,
                            x: gx,
                            y: *baseline as f32,
                        })
                        .collect();
                    vs.draw_glyphs(&self.font)
                        .font_size(*px)
                        .hint(false)
                        .brush(color(*c))
                        .draw(Fill::NonZero, glyphs.into_iter());
                }
            }
        }
        Ok(vs)
    }

    /// Renders `scene` with antialiasing `aa` and reads back straight RGBA8 (blocking).
    ///
    /// # Errors
    /// GPU submission or readback failure.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn render(&mut self, scene: &SceneDesc, aa: AaConfig) -> Result<Vec<u8>, RenderError> {
        pollster::block_on(self.render_async(scene, aa))
    }

    /// Async render + readback (works on native and on wasm32/WebGPU).
    ///
    /// # Errors
    /// GPU submission or readback failure.
    pub async fn render_async(
        &mut self,
        scene: &SceneDesc,
        aa: AaConfig,
    ) -> Result<Vec<u8>, RenderError> {
        let vs = self.encode(scene)?;
        let (w, h) = (scene.width, scene.height);
        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qp-target"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let params = RenderParams {
            base_color: color(scene.background),
            width: w,
            height: h,
            antialiasing_method: aa,
        };
        self.renderer
            .render_to_texture(&self.device, &self.queue, &vs, &view, &params)
            .map_err(err)?;

        let row = w * 4;
        let padded = row.div_ceil(256) * 256;
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qp-readback"),
            size: u64::from(padded) * u64::from(h),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(h),
                },
            },
            size,
        );
        self.queue.submit([enc.finish()]);
        let (tx, rx) = oneshot();
        buf.map_async(wgpu::MapMode::Read, .., move |r| {
            tx.send(r.map_err(|e| e.to_string()))
        });
        // Native: drive the device until the map callback has fired. WebGPU: no-op; the
        // callback arrives from the browser event loop while we await below.
        #[cfg(not(target_arch = "wasm32"))]
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(err)?;
        rx.await.map_err(err)?;
        let data = buf.get_mapped_range(..).map_err(err)?;
        let mut out = Vec::with_capacity((row * h) as usize);
        for y in 0..h as usize {
            let start = y * padded as usize;
            out.extend_from_slice(&data[start..start + row as usize]);
        }
        drop(data);
        buf.unmap();
        Ok(crate::cpu::demultiply(&out))
    }
}

/// Minimal single-value channel whose receiver is a `Future` (no extra dependencies).
struct Slot<T> {
    value: Option<T>,
    waker: Option<Waker>,
}

struct Tx<T>(Arc<Mutex<Slot<T>>>);
struct Rx<T>(Arc<Mutex<Slot<T>>>);

fn oneshot<T>() -> (Tx<T>, Rx<T>) {
    let s = Arc::new(Mutex::new(Slot {
        value: None,
        waker: None,
    }));
    (Tx(Arc::clone(&s)), Rx(s))
}

impl<T> Tx<T> {
    fn send(self, v: T) {
        let waker = {
            let mut s = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            s.value = Some(v);
            s.waker.take()
        };
        if let Some(w) = waker {
            w.wake();
        }
    }
}

impl<T> Future for Rx<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut s = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match s.value.take() {
            Some(v) => Poll::Ready(v),
            None => {
                s.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
