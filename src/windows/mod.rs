/*
 * Copyright (c) Axe. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 */

use std::sync::Arc;
use winit::platform::windows::WindowBuilderExtWindows;
use winit::window::Icon;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{CursorIcon, ResizeDirection, WindowBuilder},
};
mod utils;

const BORDER_SIZE: f64 = 8.0;

#[cfg(target_os = "windows")]
use winit::platform::windows::IconExtWindows;

pub fn start() {
    let event_loop = EventLoop::new().unwrap();
    #[cfg(target_os = "windows")]
    let icon = Icon::from_resource(1, None).unwrap();

    let window = WindowBuilder::new()
        .with_title("Ninny")
        .with_decorations(false)
        .with_window_icon(Some(icon.clone()))
        .with_taskbar_icon(Some(icon))
        .with_inner_size(winit::dpi::LogicalSize::new(1000.0, 700.0))
        .build(&event_loop)
        .unwrap();

    #[cfg(target_os = "windows")]
    utils::apply_chromium_dwm_shadow(&window);

    let window = Arc::new(window);

    // 1. Initialisation GPU (DirectX 12 / Vulkan via wgpu, exactement comme Chrome)
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12, // évite le loader Vulkan et ses layers
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let surface = instance.create_surface(window.clone()).unwrap();

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
        ..Default::default()
    }))
    .expect("Impossible de trouver un adaptateur GPU");

    // 3. Demander au device d'économiser la mémoire
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("Device GPU"),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        ..Default::default()
    }))
    .expect("Impossible de créer le device GPU");

    // 2. Configuration de la surface avec sRGB matériel et VSync 60Hz/144Hz
    let size = window.inner_size();
    let mut config = surface
        .get_default_config(&adapter, size.width.max(1), size.height.max(1))
        .expect("Surface non supportée");

    // VSync matériel sans scintillement (comme Chrome)
    config.present_mode = wgpu::PresentMode::AutoVsync;

    surface.configure(&device, &config);
    let mut resize_direction: Option<winit::window::ResizeDirection> = None;
    // 3. Boucle d'événements
    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),

                WindowEvent::Resized(new_size) => {
                    if new_size.width > 0 && new_size.height > 0 {
                        config.width = new_size.width;
                        config.height = new_size.height;
                        surface.configure(&device, &config);
                        window.request_redraw();
                    }
                }

                // Gestion de souris
                WindowEvent::CursorMoved {
                    position,
                    device_id: _,
                } => {
                    let width = config.width as f64;
                    let height = config.height as f64;

                    let on_left = position.x < BORDER_SIZE;
                    let on_right = position.x > width - BORDER_SIZE;
                    let on_top = position.y < BORDER_SIZE;
                    let on_bottom = position.y > height - BORDER_SIZE;

                    let (icon, dir) = if on_top && on_left {
                        (CursorIcon::NwseResize, Some(ResizeDirection::NorthWest))
                    } else if on_top && on_right {
                        (CursorIcon::NeswResize, Some(ResizeDirection::NorthEast))
                    } else if on_bottom && on_left {
                        (CursorIcon::NeswResize, Some(ResizeDirection::SouthWest))
                    } else if on_bottom && on_right {
                        (CursorIcon::NwseResize, Some(ResizeDirection::SouthEast))
                    } else if on_left {
                        (CursorIcon::EwResize, Some(ResizeDirection::West))
                    } else if on_right {
                        (CursorIcon::EwResize, Some(ResizeDirection::East))
                    } else if on_top {
                        (CursorIcon::NsResize, Some(ResizeDirection::North))
                    } else if on_bottom {
                        (CursorIcon::NsResize, Some(ResizeDirection::South))
                    } else {
                        (CursorIcon::Default, None)
                    };

                    // On applique l'icône de souris et on met à jour la variable pour le clic !
                    window.set_cursor_icon(icon);
                    resize_direction = dir;
                }

                WindowEvent::MouseInput {
                    state: winit::event::ElementState::Pressed,
                    button: winit::event::MouseButton::Left,
                    ..
                } => {
                    if let Some(dir) = resize_direction {
                        let _ = window.drag_resize_window(dir);
                    }
                }

                WindowEvent::RedrawRequested => {
                    #[cfg(target_os = "windows")]
                    utils::apply_chromium_dwm_shadow(&window);

                    match surface.get_current_texture() {
                        wgpu::CurrentSurfaceTexture::Success(output)
                        | wgpu::CurrentSurfaceTexture::Suboptimal(output) => {
                            let view = output
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let mut encoder =
                                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Render Encoder"),
                                });

                            let background_color = utils::srgb(23, 23, 23);

                            {
                                let _render_pass =
                                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("Render Pass"),
                                        color_attachments: &[Some(
                                            wgpu::RenderPassColorAttachment {
                                                view: &view,
                                                resolve_target: None,
                                                ops: wgpu::Operations {
                                                    load: wgpu::LoadOp::Clear(background_color),
                                                    store: wgpu::StoreOp::Store,
                                                },
                                                depth_slice: None,
                                            },
                                        )],
                                        ..Default::default()
                                    });
                            }

                            queue.submit(std::iter::once(encoder.finish()));
                            queue.present(output);
                        }
                        wgpu::CurrentSurfaceTexture::Outdated
                        | wgpu::CurrentSurfaceTexture::Lost => {
                            surface.configure(&device, &config);
                        }
                        _ => (),
                    };
                }

                _ => (),
            },
            _ => (),
        }
    });
}
