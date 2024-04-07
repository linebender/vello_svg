use std::{num::NonZeroUsize, sync::Arc};

use vello::{
    peniko::Color,
    util::{RenderContext, RenderSurface},
    AaConfig, Renderer, RendererOptions, Scene,
};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

// Simple struct to hold the state of the renderer
struct ActiveRenderState<'s> {
    // The fields MUST be in this order, so that the surface is dropped before the window
    surface: RenderSurface<'s>,
    window: Arc<Window>,
}

enum RenderState<'s> {
    Active(ActiveRenderState<'s>),
    // Cache a window so that it can be reused when the app is resumed after being suspended
    Suspended(Option<Arc<Window>>),
}

pub fn display(width: u32, height: u32, scene: Scene) -> anyhow::Result<()> {
    // The vello RenderContext which is a global context that lasts for the lifetime of the application
    let mut render_cx = RenderContext::new().unwrap();

    // An array of renderers, one per wgpu device
    let mut renderers: Vec<Option<Renderer>> = vec![];

    // State for our example where we store the winit Window and the wgpu Surface
    let mut render_state = RenderState::Suspended(None);

    // Create and run a winit event loop
    let event_loop = EventLoop::new()?;
    event_loop
        .run(move |event, event_loop| match event {
            // Setup renderer. In winit apps it is recommended to do setup in Event::Resumed
            // for best cross-platform compatibility
            Event::Resumed => {
                let RenderState::Suspended(cached_window) = &mut render_state else {
                    return;
                };

                // Get the winit window cached in a previous Suspended event or else create a new window
                let window = cached_window.take().unwrap_or_else(|| {
                    Arc::new(
                        WindowBuilder::new()
                            .with_inner_size(LogicalSize::new(width, height))
                            .with_resizable(false)
                            .with_title("Vello SVG Example Display")
                            .build(event_loop)
                            .unwrap(),
                    )
                });

                // Create a vello Surface
                let size = window.inner_size();
                let surface_future = render_cx.create_surface(
                    window.clone(),
                    size.width,
                    size.height,
                    wgpu::PresentMode::AutoVsync,
                );
                let surface = pollster::block_on(surface_future).expect("Error creating surface");

                // Create a vello Renderer for the surface (using its device id)
                renderers.resize_with(render_cx.devices.len(), || None);
                renderers[surface.dev_id].get_or_insert_with(|| {
                    Renderer::new(
                        &render_cx.devices[surface.dev_id].device,
                        RendererOptions {
                            surface_format: Some(surface.format),
                            use_cpu: false,
                            antialiasing_support: vello::AaSupport::all(),
                            num_init_threads: NonZeroUsize::new(1),
                        },
                    )
                    .expect("Couldn't create renderer")
                });

                // Save the Window and Surface to a state variable
                render_state = RenderState::Active(ActiveRenderState { window, surface });

                event_loop.set_control_flow(ControlFlow::Poll);
            }

            // Save window state on suspend
            Event::Suspended => {
                if let RenderState::Active(state) = &render_state {
                    render_state = RenderState::Suspended(Some(state.window.clone()));
                }
                event_loop.set_control_flow(ControlFlow::Wait);
            }

            Event::WindowEvent {
                ref event,
                window_id,
            } => {
                // Ignore the event (return from the function) if
                //   - we have no render_state
                //   - OR the window id of the event doesn't match the window id of our render_state
                //
                // Else extract a mutable reference to the render state from its containing option for use below
                let render_state = match &mut render_state {
                    RenderState::Active(state) if state.window.id() == window_id => state,
                    _ => return,
                };

                match event {
                    // Exit the event loop when a close is requested (e.g. window's close button is pressed)
                    WindowEvent::CloseRequested => event_loop.exit(),

                    // This is where all the rendering happens
                    WindowEvent::RedrawRequested => {
                        // Get the RenderSurface (surface + config)
                        let surface = &render_state.surface;

                        // Get the window size
                        let width = surface.config.width;
                        let height = surface.config.height;

                        // Get a handle to the device
                        let device_handle = &render_cx.devices[surface.dev_id];

                        // Get the surface's texture
                        let surface_texture = surface
                            .surface
                            .get_current_texture()
                            .expect("failed to get surface texture");

                        // Render to the surface's texture
                        renderers[surface.dev_id]
                            .as_mut()
                            .unwrap()
                            .render_to_surface(
                                &device_handle.device,
                                &device_handle.queue,
                                &scene,
                                &surface_texture,
                                &vello::RenderParams {
                                    base_color: Color::BLACK, // Background color
                                    width,
                                    height,
                                    antialiasing_method: AaConfig::Msaa16,
                                },
                            )
                            .expect("failed to render to surface");

                        // Queue the texture to be presented on the surface
                        surface_texture.present();

                        device_handle.device.poll(wgpu::Maintain::Poll);
                    }
                    _ => {}
                }
            }
            _ => {}
        })
        .expect("Couldn't run event loop");
    Ok(())
}
