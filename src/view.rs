use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

use gameboy::{device::Device, memory::mmu::JoypadButton};
use glium::{
    glutin::{
        dpi::LogicalSize,
        event::{ElementState, Event, VirtualKeyCode, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
        ContextBuilder,
    },
    texture::{ClientFormat, MipmapsOption, RawImage2d, UncompressedFloatFormat},
    uniforms::MagnifySamplerFilter,
    BlitTarget, Display, Rect, Surface, Texture2d,
};

pub fn start_view(mut device: Device) {
    let event_loop = EventLoop::new();
    let context = ContextBuilder::new().with_vsync(true);
    let builder = WindowBuilder::new()
        .with_title(device.cart().title().unwrap_or("gameboy"))
        .with_inner_size(LogicalSize::new(160 * 3, 144 * 3));
    let display = Display::new(builder, context, &event_loop).expect("failed to create display");

    let texture = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::U8U8U8,
        MipmapsOption::NoMipmap,
        160,
        144,
    )
    .expect("failed to create display texture");

    let emulation_speed = 4194304.0 / 70224.0;
    let mut last_frame = Instant::now();

    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            let gl_window = display.gl_window();
            gl_window.window().request_redraw();
        }
        Event::RedrawRequested(_) => {
            if last_frame.elapsed().as_secs_f32() >= 1.0 / emulation_speed {
                last_frame += Duration::from_secs_f32(1.0 / emulation_speed);
                device.step_frame();
            }

            let framebuffer = device.display_framebuffer();

            texture.write(
                Rect {
                    left: 0,
                    bottom: 0,
                    width: 160,
                    height: 144,
                },
                RawImage2d {
                    data: Cow::Borrowed(framebuffer),
                    width: 160,
                    height: 144,
                    format: ClientFormat::U8U8U8,
                },
            );

            let target = display.draw();
            let (target_w, target_h) = target.get_dimensions();
            texture.as_surface().blit_whole_color_to(
                &target,
                &BlitTarget {
                    left: 0,
                    bottom: target_h,
                    width: target_w as i32,
                    height: -(target_h as i32),
                },
                MagnifySamplerFilter::Nearest,
            );
            target.finish().unwrap();
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } => {
            let button = match input.virtual_keycode {
                Some(VirtualKeyCode::Left) => JoypadButton::Left,
                Some(VirtualKeyCode::Right) => JoypadButton::Right,
                Some(VirtualKeyCode::Up) => JoypadButton::Up,
                Some(VirtualKeyCode::Down) => JoypadButton::Down,
                Some(VirtualKeyCode::Z) => JoypadButton::B,
                Some(VirtualKeyCode::X) => JoypadButton::A,
                Some(VirtualKeyCode::LControl) => JoypadButton::Start,
                Some(VirtualKeyCode::LShift) => JoypadButton::Select,
                _ => return,
            };

            match input.state {
                ElementState::Pressed => device.press(&[button]),
                ElementState::Released => device.release(&[button]),
            }
        }
        _ => {}
    });
}
