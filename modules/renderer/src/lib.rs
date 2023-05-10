#![feature(thread_local)]

use std::cell::RefCell;
use std::rc::Rc;
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
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertices: Vec<Vertex>,
    array: Vec<u16>,
    window: Window,
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
            window,
            surface,
            device,
            queue,
            config,
            size,
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

struct RenderLoop {
    animation_id: Option<i32>,
    closure: Option<Closure<dyn FnMut()>>,
}

impl RenderLoop {
    fn new() -> Self {
        Self { animation_id: None, closure: None }
    }
}

macro_rules! animation {
    ($constructor:expr, $animation:ident, $state:ident, $speed:ident, $end:ident, $block:block) => {
        let (sx, rx) = oneshot::channel();
        let animation = Rc::new(RefCell::new($constructor));
        {
            let closure: Closure<dyn FnMut()> = {
                let mut sx = Some(sx);
                let animation = animation.clone();
                Closure::wrap(Box::new(move || {
                    let mut $animation = animation.borrow_mut();
                    let $state = unsafe { STATE.as_mut().unwrap() };
                    let mut $end = || {
                        if let Some(sx) = sx.take() {
                            sx.send(()).unwrap();
                        }
                    };
                    for _ in 0..$speed {
                        $block;
                    }
                    $state.render_array();
                    $animation.render_loop.animation_id = if let Some(ref closure) = $animation.render_loop.closure {
                        Some(web_sys::window().unwrap().request_animation_frame(closure.as_ref().unchecked_ref()).unwrap())
                    } else {
                        None
                    }
                }))
            };
            let mut animation = animation.borrow_mut();
            animation.render_loop.animation_id = Some(web_sys::window().unwrap().request_animation_frame(closure.as_ref().unchecked_ref()).unwrap());
            animation.render_loop.closure = Some(closure);
        }
        rx.await.unwrap();
    };
}

#[wasm_bindgen]
pub async fn bubble_sort(speed: u16) {
    struct BubbleSort {
        render_loop: RenderLoop,
        cycle: usize,
        swaps: usize,
        index: usize,
    }

    impl BubbleSort {
        fn new() -> Self {
            BubbleSort { render_loop: RenderLoop::new(), cycle: 0, swaps: 0, index: 1 }
        }
    }

    animation!(BubbleSort::new(), animation, state, speed, end, {
        if animation.cycle == state.array.len() {
            end();
            state.render_array();
            return;
        }

        if animation.index == state.array.len() {
            if animation.swaps == 0 {
                end();
                state.render_array();
                return;
            }

            animation.cycle += 1;
            animation.index = 1;
        }

        if state.array[animation.index - 1] > state.array[animation.index] {
            state.array.swap(animation.index - 1, animation.index);

            animation.swaps += 1;
        }

        animation.index += 1;
    });
}

#[wasm_bindgen]
pub async fn bi_bubble_sort(speed: u16) {
    struct BiBubbleSort {
        render_loop: RenderLoop,
        cycle: usize,
        swaps: usize,
        index: [usize; 2],
    }

    impl BiBubbleSort {
        fn new() -> Self {
            Self { render_loop: RenderLoop::new(), cycle: 0, swaps: 0, index: [0, 799] }
        }
    }

    let speed = speed / 2;

    animation!(BiBubbleSort::new(), animation, state, speed, end, {
        if animation.cycle == state.array.len() {
            end();
            state.render_array();
            return;
        }

        if animation.index[0] == state.array.len() - 1 || animation.index[1] == 1 {
            if animation.swaps == 0 {
                end();
                state.render_array();
                return;
            }

            animation.cycle += 1;
            animation.index = [0, state.array.len() - 1];
        }

        let index = animation.index[0];

        if state.array[index] > state.array[index + 1] {
            state.array.swap(index, index + 1);

            animation.swaps += 1;
        }

        animation.index[0] += 1;

        let index = animation.index[1];

        if state.array[index] < state.array[index - 1] {
            state.array.swap(index, index - 1);

            animation.swaps += 1;
        }

        animation.index[1] -= 1;
    });
}

#[wasm_bindgen]
pub async fn insertion_sort(speed: u16) {
    struct InsertionSort {
        render_loop: RenderLoop,
        i: usize,
        j: usize,
        f: bool,
    }

    impl InsertionSort {
        fn new() -> Self {
            Self { render_loop: RenderLoop::new(), i: 1, j: 0, f: true }
        }
    }

    animation!(InsertionSort::new(), animation, state, speed, end, {
        if animation.i < state.array.len() {
            if animation.f {
                animation.j = animation.i;
                animation.f = false;
            }

            if animation.j > 0 && state.array[animation.j - 1] > state.array[animation.j] {
                state.array.swap(animation.j, animation.j - 1);

                animation.j -= 1;
            } else {
                animation.i += 1;
                animation.f = true;
            }
        } else {
            end();
            state.render_array();
            return;
        }
    });
}
