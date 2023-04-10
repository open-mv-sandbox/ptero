use std::borrow::Cow;

use anyhow::Error;
use ptero_daicon::{OpenMode, SourceAction, SourceMessage};
use ptero_file::ReadResult;
use stewart::{Actor, Addr, After, Context, Id, Options, System};
use stewart_utils::WhenExt;
use tracing::{event, instrument, Level};
use uuid::{uuid, Uuid};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::{
    Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, FragmentState, Instance,
    Limits, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor, PowerPreference,
    PresentMode, PrimitiveState, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModuleDescriptor,
    ShaderSource, Surface, SurfaceConfiguration, TextureUsages, TextureViewDescriptor, VertexState,
};

use crate::surface;

/// JS browser viewer instance handle.
#[wasm_bindgen]
pub struct Viewer {
    system: System,
    addr: Addr<Message>,
}

#[wasm_bindgen]
impl Viewer {
    pub async fn from_canvas(target: HtmlCanvasElement) -> Result<Viewer, String> {
        crate::init_hooks();

        let mut system = System::new();
        let mut ctx = Context::root(&mut system);
        let addr = start_service(&mut ctx, target)
            .await
            .map_err(|v| v.to_string())?;

        system.run_until_idle().map_err(|v| v.to_string())?;

        Ok(Viewer { system, addr })
    }

    pub fn tick(&mut self) -> Result<(), String> {
        self.system.send(self.addr, Message::Tick);
        self.system.run_until_idle().map_err(|v| v.to_string())?;
        Ok(())
    }
}

#[instrument("viewer-service", skip_all)]
async fn start_service(
    ctx: &mut Context<'_>,
    target: HtmlCanvasElement,
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

    // Load the shaders from disk
    let shader_str = include_str!("../../../data/shader.wgsl");
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(Cow::Borrowed(shader_str)),
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

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
        render_pipeline,
    };
    let (id, mut ctx) = ctx.create()?;
    ctx.start(id, Options::default(), service)?;

    // Start a fetch request for shader data
    // TODO: Actually fetch, this is a placeholder
    let buffer =
        include_bytes!("../../../packages/dacti-example-web/public/viewer-builtins.dacti-pack")
            .to_vec();
    let file = ptero_file::open_buffer(&mut ctx, buffer)?;
    let source = ptero_daicon::open_file(&mut ctx, file, OpenMode::ReadWrite)?;

    let on_result = ctx.when(|_system, message: ReadResult| {
        let shader = std::str::from_utf8(&message.data)?;
        event!(Level::INFO, "received\n{}", shader);
        Ok(After::Stop)
    })?;
    let action = SourceAction::Get {
        id: uuid!("bacc2ba1-8dc7-4d54-a7a4-cdad4d893a1b"),
        on_result,
    };
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action,
    };
    ctx.send(source, message);

    Ok(Addr::new(id))
}

struct ViewerService {
    surface: Surface,
    device: Device,
    queue: Queue,
    render_pipeline: RenderPipeline,
}

impl ViewerService {
    fn tick(&mut self) {
        let frame = self.surface.get_current_texture().unwrap();
        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
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
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

impl Actor for ViewerService {
    type Message = Message;

    fn handle(&mut self, _system: &mut System, _id: Id, message: Message) -> Result<After, Error> {
        match message {
            Message::ShaderFetched(_data) => {
                // TODO
            }
            Message::Tick => self.tick(),
        }

        Ok(After::Continue)
    }
}

enum Message {
    ShaderFetched(String),
    Tick,
}
