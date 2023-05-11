#![feature(thread_local)]

use rand::prelude::SliceRandom;
use rand::thread_rng;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use winit::platform::web::WindowExtWebSys;

use wasm_bindgen::prelude::*;

#[thread_local]
static mut STATE: Option<State> = None;

#[wasm_bindgen(start)]
pub async fn run() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Renderer")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();

    {
        let main = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("m"))
            .unwrap();

        let canvas = window.canvas();

        main.append_child(&canvas).unwrap();
    }

    let mut state = State::new(window).await;

    state.render().unwrap();

    unsafe { STATE.replace(state); }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    _config: wgpu::SurfaceConfiguration,
    _size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertices: Vec<Vertex>,
    array: Vec<u16>,
    _window: Window,
}

impl State {
    async fn new(window: Window) -> Self {
        use wgpu::util::DeviceExt;

        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_webgl2_defaults(),
                label: None,
            },
            None,
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps.formats.iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let mut array = Vec::with_capacity(800);

        for i in 0..800 {
            array.push(i + 1);
        }

        let mut vertices = Vec::with_capacity(1600);

        fill_vertices(&array, &mut vertices);

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(vertices.as_slice()),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );

        Self {
            _window: window,
            surface,
            device,
            queue,
            _config: config,
            _size: size,
            render_pipeline,
            vertex_buffer,
            vertices,
            array,
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..1600, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        output.present();

        Ok(())
    }

    fn render_array(&mut self) {
        fill_vertices(&self.array, &mut self.vertices);
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(self.vertices.as_slice()));
        self.render().unwrap();
    }
}

fn fill_vertices(array: &Vec<u16>, vertices: &mut Vec<Vertex>) {
    use map_range::MapRange;

    vertices.clear();

    const PIXEL_H: f32 = 2.0 / 400.0;
    const PIXEL_W: f32 = 2.0 / 800.0;

    let mut x = -1.0;

    for e in array.iter().copied() {
        vertices.push(Vertex { position: [x, -1.0] });

        let y = -1.0 + PIXEL_H * (e as f32).map_range(1.0..801.0, 1.0..401.0);

        vertices.push(Vertex { position: [x, y] });

        x += PIXEL_W;
    }
}

#[wasm_bindgen]
pub fn shuffle() {
    let state = unsafe { STATE.as_mut().unwrap() };
    state.array.shuffle(&mut thread_rng());
    state.render_array();
}

#[wasm_bindgen]
pub fn reverse() {
    let state = unsafe { STATE.as_mut().unwrap() };
    state.array.reverse();
    state.render_array();
}

async fn request_animation_frame() {
    let (s, r) = oneshot::channel();

    let closure = Closure::once(move || s.send(()).unwrap());

    web_sys::window().unwrap()
        .request_animation_frame(closure.as_ref().unchecked_ref()).unwrap();

    r.await.unwrap();
}

#[wasm_bindgen]
pub async fn bubble_sort(speed: u16) {
    let state = unsafe { STATE.as_mut().unwrap() };

    let mut mods = 0;

    for i in 0..state.array.len() - 1 {
        for j in 0..state.array.len() - i - 1 {
            if state.array[j] > state.array[j + 1] {
                state.array.swap(j, j + 1);

                mods += 1;

                if mods == speed {
                    mods = 0;

                    request_animation_frame().await;

                    state.render_array();
                }
            }
        }
    }

    request_animation_frame().await;

    state.render_array();
}

#[wasm_bindgen]
pub async fn insertion_sort(speed: u16) {
    let state = unsafe { STATE.as_mut().unwrap() };

    let mut mods = 0;

    let mut i = 1;

    while i < state.array.len() {
        let mut j = i;

        while j > 0 && state.array[j - 1] > state.array[j] {
            state.array.swap(j, j - 1);

            j -= 1;

            mods += 1;

            if mods == speed {
                mods = 0;

                request_animation_frame().await;

                state.render_array();
            }
        }

        i += 1;
    }

    request_animation_frame().await;

    state.render_array();
}

#[wasm_bindgen]
pub async fn merge_sort(speed: u16) {
    let state = unsafe { STATE.as_mut().unwrap() };

    let mut mods = 0;

    let mut a = 1;
    let mut b = 2;

    loop {
        for c in state.array.chunks_mut(b) {
            if b < 2 {
                continue;
            }

            let mut temp = Vec::with_capacity(c.len());

            {
                let mid = b / 2;

                let mut lhs = c.iter().take(mid).peekable();
                let mut rhs = c.iter().skip(mid).peekable();

                while let (Some(&lhs_el), Some(&rhs_el)) = (lhs.peek(), rhs.peek()) {
                    if *lhs_el <= *rhs_el {
                        temp.push(*lhs.next().unwrap());
                    } else {
                        temp.push(*rhs.next().unwrap());
                    }
                }

                for el in lhs {
                    temp.push(*el);
                }

                for el in rhs {
                    temp.push(*el);
                }
            }

            for i in 0..temp.len() {
                c[i] = temp[i];

                mods += 1;

                if mods == speed {
                    mods = 0;

                    request_animation_frame().await;

                    unsafe { STATE.as_mut().unwrap().render_array() };
                }
            }
        }

        if b > state.array.len() {
            break;
        }

        a += 1;

        b = 2f64.powi(a) as usize;
    }

    request_animation_frame().await;

    state.render_array();
}

#[wasm_bindgen]
pub async fn quick_sort(speed: u16) {
    let state = unsafe { STATE.as_mut().unwrap() };

    let mut mods = 0;

    macro_rules! on_mod {
        () => {
            mods += 1;

            if mods == speed {
                mods = 0;

                request_animation_frame().await;

                state.render_array();
            }
        };
    }

    let mut stack = Vec::with_capacity(state.array.len());

    stack.push(0);
    stack.push(state.array.len() - 1);

    while !stack.is_empty() {
        let h = stack.pop().unwrap();
        let l = stack.pop().unwrap();

        let mut i = l;

        let pivot = state.array[h];

        for j in l..h {
            if state.array[j] <= pivot {
                state.array.swap(i, j);

                i += 1;

                on_mod!();
            }
        }

        state.array.swap(i, h);

        on_mod!();

        if i > 0 && i - 1 > l {
            stack.push(l);
            stack.push(i - 1);
        }

        if i + 1 < h {
            stack.push(i + 1);
            stack.push(h);
        }
    }

    request_animation_frame().await;

    state.render_array();
}