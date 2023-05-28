use std::{borrow::Cow, cell::RefCell, rc::Rc};

use anyhow::Error;
use daicon::{
    open_file_source,
    protocol::{ReadResult, SourceAction, SourceGet, SourceMessage},
    OpenMode, OpenOptions,
};
use daicon_types::Id;
use daicon_web::{open_fetch_file, WorldHandle};
use stewart::{Actor, Context, Options, Sender, State, World};
use tracing::{event, instrument, Level};
use uuid::Uuid;
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
    hnd: WorldHandle,
    sender: Sender<Message>,
}

#[wasm_bindgen]
impl Viewer {
    pub async fn from_canvas(target: HtmlCanvasElement) -> Result<Viewer, String> {
        crate::init_hooks();

        let world = World::new();
        let hnd = Rc::new(RefCell::new(world));

        let sender = start_service(hnd.clone(), target)
            .await
            .map_err(|v| v.to_string())?;

        let mut world = hnd.borrow_mut();
        world.run_until_idle().map_err(|v| v.to_string())?;
        drop(world);

        Ok(Viewer { hnd, sender })
    }

    pub fn tick(&mut self) -> Result<(), String> {
        let mut world = self.hnd.borrow_mut();
        let mut ctx = world.root();

        self.sender.send(&mut ctx, Message::Tick);
        world.run_until_idle().map_err(|v| v.to_string())?;
        Ok(())
    }
}

#[instrument("viewer-service", skip_all)]
async fn start_service(
    hnd: WorldHandle,
    target: HtmlCanvasElement,
) -> Result<Sender<Message>, Error> {
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

    let mut world = hnd.borrow_mut();
    let mut ctx = world.root();

    let (mut ctx, sender) = ctx.create(Options::default())?;
    ctx.start(service)?;

    // Start a fetch request for shader data
    let file = open_fetch_file(
        &mut ctx,
        "/viewer-builtins.dacti-pack".to_string(),
        hnd.clone(),
    )?;
    let source = open_file_source(&mut ctx, file, OpenMode::ReadWrite, OpenOptions::default())?;

    let action = SourceGet {
        id: Id(0xbacc2ba1),
        on_result: sender.clone().map(Message::ShaderFetched),
    };
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Get(action),
    };
    source.send(&mut ctx, message);

    // Just for testing, fetch an additional resource
    let action = SourceGet {
        id: Id(0x1f063ad4),
        on_result: Sender::noop(),
    };
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Get(action),
    };
    source.send(&mut ctx, message);

    Ok(sender)
}

struct ViewerService {
    surface: Surface,
    device: Device,
    queue: Queue,

    pipeline_layout: PipelineLayout,
    swapchain_format: TextureFormat,
    render_pipeline: Option<RenderPipeline>,
}

impl Actor for ViewerService {
    type Message = Message;

    fn process(&mut self, _ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                Message::ShaderFetched(message) => {
                    let data = std::str::from_utf8(&message.data)?;
                    event!(Level::INFO, "received shader\n{}", data);

                    let pipeline = create_render_pipeline(
                        &self.device,
                        &self.pipeline_layout,
                        self.swapchain_format,
                        &data,
                    );
                    self.render_pipeline = Some(pipeline);
                }
                Message::Tick => self.tick(),
            }
        }

        Ok(())
    }
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
