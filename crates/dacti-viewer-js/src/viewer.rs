use std::{borrow::Cow, cell::RefCell, rc::Rc};

use anyhow::Error;
use ptero_daicon::{OpenMode, SourceAction, SourceMessage};
use ptero_file::ReadResult;
use ptero_js::SystemH;
use stewart::{Addr, State, System, World};
use stewart_utils::{Context, Functional};
use tracing::{event, instrument, Level};
use uuid::{uuid, Uuid};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::{
    Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, FragmentState, Instance,
    Limits, LoadOp, MultisampleState, Operations, PipelineLayout, PipelineLayoutDescriptor,
    PowerPreference, PresentMode, PrimitiveState, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderModuleDescriptor, ShaderSource, Surface, SurfaceConfiguration, TextureFormat,
    TextureUsages, TextureViewDescriptor, VertexState,
};

use crate::surface;

/// JS browser viewer instance handle.
#[wasm_bindgen]
pub struct Viewer {
    hnd: SystemH,
    addr: Addr<Message>,
}

#[wasm_bindgen]
impl Viewer {
    pub async fn from_canvas(target: HtmlCanvasElement) -> Result<Viewer, String> {
        crate::init_hooks();

        let system = World::new();
        let hnd = Rc::new(RefCell::new(system));
        let mut system = hnd.borrow_mut();

        let mut ctx = Context::root(&mut system);
        let addr = start_service(&mut ctx, target, hnd.clone())
            .await
            .map_err(|v| v.to_string())?;

        system.run_until_idle().map_err(|v| v.to_string())?;
        drop(system);

        Ok(Viewer { hnd, addr })
    }

    pub fn tick(&mut self) -> Result<(), String> {
        let mut system = self.hnd.borrow_mut();
        system.send(self.addr, Message::Tick);
        system.run_until_idle().map_err(|v| v.to_string())?;
        Ok(())
    }
}

#[instrument("viewer-service", skip_all)]
async fn start_service(
    ctx: &mut Context<'_>,
    target: HtmlCanvasElement,
    hnd: SystemH,
) -> Result<Addr<Message>, Error> {
    event!(Level::INFO, "creating viewer service");

    let instance = Instance::default();

    // Create the surface
    let target: HtmlCanvasElement = target
        .dyn_into()
        .expect("given target is not a canvas element");
    let surface = surface::create(&instance, &target);

    // Create the adapter
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::default(),
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .unwrap();

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: None,
                features: Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                limits: Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .unwrap();

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: 800,
        height: 600,
        present_mode: PresentMode::Fifo,
        alpha_mode: swapchain_capabilities.alpha_modes[0],
        view_formats: vec![],
    };

    surface.configure(&device, &config);

    // Start the service
    let service = ViewerService {
        surface,
        device,
        queue,

        pipeline_layout,
        swapchain_format,
        render_pipeline: None,
    };

    let id = ctx.register(ViewerServiceSystem);

    let (id, mut ctx) = ctx.create(id)?;
    ctx.start(id, service)?;
    let addr = Addr::new(id);

    // Start a fetch request for shader data
    let file = ptero_js::open_fetch_file(&mut ctx, "/viewer-builtins.dacti-pack".to_string(), hnd)?;
    let source = ptero_daicon::open_file(&mut ctx, file, OpenMode::ReadWrite)?;

    let action = SourceAction::Get {
        id: uuid!("bacc2ba1-8dc7-4d54-a7a4-cdad4d893a1b"),
        on_result: ctx.map_once(addr, Message::ShaderFetched)?,
    };
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action,
    };
    ctx.send(source, message);

    // Just for testing, fetch an additional resource
    let action = SourceAction::Get {
        id: uuid!("1f063ad4-5a91-47fe-b95c-668fc41a719d"),
        on_result: ctx.when(|_, _, _| Ok(false))?,
    };
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action,
    };
    ctx.send(source, message);

    Ok(addr)
}

struct ViewerServiceSystem;

impl System for ViewerServiceSystem {
    type Instance = ViewerService;
    type Message = Message;

    fn process(&mut self, _world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some((_id, instance, message)) = state.next() {
            match message {
                Message::ShaderFetched(message) => {
                    let data = std::str::from_utf8(&message.data)?;
                    let pipeline = create_render_pipeline(
                        &instance.device,
                        &instance.pipeline_layout,
                        instance.swapchain_format,
                        &data,
                    );
                    instance.render_pipeline = Some(pipeline);
                }
                Message::Tick => instance.tick(),
            }
        }

        Ok(())
    }
}

struct ViewerService {
    surface: Surface,
    device: Device,
    queue: Queue,

    pipeline_layout: PipelineLayout,
    swapchain_format: TextureFormat,
    render_pipeline: Option<RenderPipeline>,
}

impl ViewerService {
    fn tick(&mut self) {
        let frame = self.surface.get_current_texture().unwrap();
        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        if let Some(render_pipeline) = &self.render_pipeline {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::GREEN),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(render_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

enum Message {
    ShaderFetched(ReadResult),
    Tick,
}

fn create_render_pipeline(
    device: &Device,
    pipeline_layout: &PipelineLayout,
    swapchain_format: TextureFormat,
    shader_str: &str,
) -> RenderPipeline {
    event!(Level::INFO, "creating render pipeline");

    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(Cow::Borrowed(shader_str)),
    });
    let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(swapchain_format.into())],
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview: None,
    });

    render_pipeline
}
