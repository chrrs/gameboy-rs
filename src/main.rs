use std::{fs::File, ptr::null};

use clap::{App, Arg};
use gameboy::{cartridge::Cartridge, cpu::CpuFlag, device::Device};
use glium::{
    glutin::{
        dpi::LogicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
        ContextBuilder,
    },
    Display, Surface,
};
use imgui::{
    im_str,
    sys::{igBeginPopupContextItem, igEndPopup},
    Context, FontConfig, FontSource, ImString, Key, MenuItem, Selectable, Window,
};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};

fn main() {
    let matches = App::new("gameboy")
        .about("A simple non-color gameboy emulator")
        .arg(
            Arg::new("rom")
                .index(1)
                .required(true)
                .about("The gameboy ROM file to load"),
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .about("Activates the extra debugging window"),
        )
        .get_matches();

    let cart = Cartridge::new(File::open(matches.value_of("rom").unwrap()).unwrap()).unwrap();
    let mut device = Device::new(cart);

    let disassembly = device.disassemble(0x8000);

    let event_loop = EventLoop::new();
    let context = ContextBuilder::new().with_vsync(true);
    let builder = WindowBuilder::new()
        .with_title(device.cart().title().unwrap_or("gameboy"))
        .with_inner_size(LogicalSize::new(800, 600));
    let display = Display::new(builder, context, &event_loop).unwrap();

    let mut imgui = Context::create();
    imgui.set_ini_filename(None);

    let mut platform = WinitPlatform::init(&mut imgui);
    {
        let gl_window = display.gl_window();
        let window = gl_window.window();
        platform.attach_window(imgui.io_mut(), window, HiDpiMode::Default);
    }

    let hidpi_factor = platform.hidpi_factor();
    let font_size = hidpi_factor * 13.0;
    imgui.fonts().add_font(&[FontSource::DefaultFontData {
        config: Some(FontConfig {
            size_pixels: font_size as f32,
            ..FontConfig::default()
        }),
    }]);

    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

    let mut renderer = Renderer::init(&mut imgui, &display).unwrap();

    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            let gl_window = display.gl_window();
            platform
                .prepare_frame(imgui.io_mut(), gl_window.window())
                .unwrap();
            gl_window.window().request_redraw();
        }
        Event::RedrawRequested(_) => {
            let ui = imgui.frame();

            Window::new(im_str!("CPU State")).build(&ui, || {
                let flag_color = |set| {
                    if set {
                        [0.0, 1.0, 0.0, 1.0]
                    } else {
                        [1.0, 0.0, 0.0, 1.0]
                    }
                };

                ui.text_colored(flag_color(device.cpu().get_flag(CpuFlag::Zero)), "Z");
                ui.same_line_with_spacing(0.0, 8.0);
                ui.text_colored(flag_color(device.cpu().get_flag(CpuFlag::Subtraction)), "S");
                ui.same_line_with_spacing(0.0, 8.0);
                ui.text_colored(flag_color(device.cpu().get_flag(CpuFlag::HalfCarry)), "H");
                ui.same_line_with_spacing(0.0, 8.0);
                ui.text_colored(flag_color(device.cpu().get_flag(CpuFlag::Carry)), "C");

                ui.separator();

                ui.text(format!("PC: {:#06x}", device.cpu().pc));
                ui.text(format!("SP: {:#06x}", device.cpu().sp));
                ui.spacing();
                ui.text(format!("Scanline: {}", device.gpu().scanline()));
                ui.text(format!(
                    "Scroll: {}, {}",
                    device.gpu().scroll_x,
                    device.gpu().scroll_y
                ));
                ui.spacing();
                ui.text(format!("AF: {0:#06x} ({0})", device.cpu().af()));
                ui.text(format!("BC: {0:#06x} ({0})", device.cpu().bc()));
                ui.text(format!("DE: {0:#06x} ({0})", device.cpu().de()));
                ui.text(format!("HL: {0:#06x} ({0})", device.cpu().hl()));

                ui.separator();

                if ui.button(im_str!("Reset device"), [0.0, 0.0]) {
                    device.reset();
                }
            });

            Window::new(im_str!("Disassembly")).build(&ui, || {
                let pc_bound = device.cpu().pc.saturating_sub(20);

                disassembly
                    .iter()
                    .skip_while(|(addr, _)| **addr < pc_bound)
                    .take(0x1000)
                    .for_each(|(addr, instruction)| {
                        Selectable::new(&ImString::new(format!("{:#06x}: {}", addr, instruction)))
                            .selected(&device.cpu().pc == addr)
                            .build(&ui);

                        if unsafe { igBeginPopupContextItem(null(), 0) } {
                            if MenuItem::new(im_str!("Jump to here")).build(&ui) {
                                device.cpu_mut().pc = *addr;
                            }

                            if MenuItem::new(im_str!("Run to here")).build(&ui) {
                                while device.cpu_mut().pc != *addr {
                                    device.step();
                                }
                            }

                            unsafe { igEndPopup() };
                        }
                    });
            });

            if ui.is_key_pressed(Key::Enter) {
                device.step();
            }

            let gl_window = display.gl_window();
            let mut target = display.draw();

            target.clear_color_srgb(1.0, 1.0, 1.0, 1.0);

            platform.prepare_render(&ui, gl_window.window());
            let draw_data = ui.render();
            renderer.render(&mut target, draw_data).unwrap();

            target.finish().unwrap();
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        event => platform.handle_event(imgui.io_mut(), display.gl_window().window(), &event),
    });
}
