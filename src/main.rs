use std::{borrow::Cow, fs::File, ptr::null, rc::Rc};

use clap::{App, Arg};
use gameboy::{cartridge::Cartridge, cpu::CpuFlag, device::Device};
use glium::{
    backend::Facade,
    glutin::{
        dpi::LogicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
        ContextBuilder,
    },
    texture::{ClientFormat, RawImage2d},
    uniforms::{MagnifySamplerFilter, SamplerBehavior},
    Display, Surface, Texture2d,
};
use imgui::{
    im_str,
    sys::{igBeginPopupContextItem, igEndPopup},
    ChildWindow, Condition, Context, FontConfig, FontSource, ImString, Image, MenuItem, Selectable,
    Window,
};
use imgui_glium_renderer::{Renderer, Texture};
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

    let cart = Cartridge::new(
        File::open(
            matches
                .value_of("rom")
                .expect("no rom command line argument supplied"),
        )
        .expect("file not found"),
    )
    .expect("failed to read file");
    let mut device = Device::new(cart);

    let disassembly = device.disassemble(0x8000);

    let event_loop = EventLoop::new();
    let context = ContextBuilder::new().with_vsync(true);
    let builder = WindowBuilder::new()
        .with_title(device.cart().title().unwrap_or("gameboy"))
        .with_inner_size(LogicalSize::new(874, 473));
    let display = Display::new(builder, context, &event_loop).expect("failed to create display");

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

    let mut renderer =
        Renderer::init(&mut imgui, &display).expect("failed to create imgui glium renderer");

    let data = vec![0u8; 144 * 160 * 3];
    let raw_image = RawImage2d {
        data: Cow::Owned(data),
        width: 160,
        height: 144,
        format: ClientFormat::U8U8U8,
    };

    let texture2d = Rc::new(
        Texture2d::new(display.get_context(), raw_image).expect("failed to create display texture"),
    );
    let texture_id = renderer.textures().insert(Texture {
        texture: texture2d,
        sampler: SamplerBehavior {
            magnify_filter: MagnifySamplerFilter::Nearest,
            ..SamplerBehavior::default()
        },
    });

    let mut display_scale = 3;
    let mut follow_execution = true;

    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            let gl_window = display.gl_window();
            platform
                .prepare_frame(imgui.io_mut(), gl_window.window())
                .expect("failed to prepare imgui frame");
            gl_window.window().request_redraw();
        }
        Event::RedrawRequested(_) => {
            let ui = imgui.frame();

            Window::new(im_str!("CPU State"))
                .position([206.0, 238.0], Condition::FirstUseEver)
                .size([166.0, 0.0], Condition::FirstUseEver)
                .build(&ui, || {
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
                });

            Window::new(im_str!("Device Controls"))
                .position([206.0, 3.0], Condition::FirstUseEver)
                .resizable(false)
                .build(&ui, || {
                    ui.button(im_str!("Pause"), [150.0, 0.0]);
                    ui.text("Status: Running");

                    ui.separator();

                    if ui.button(im_str!("Step instruction"), [150.0, 0.0]) {
                        device.step();
                    }

                    ui.button(im_str!("Step frame"), [150.0, 0.0]);

                    if ui.button(im_str!("Skip instruction"), [150.0, 0.0]) {
                        device.skip();
                    }

                    ui.separator();

                    ui.text(im_str!("Emulation speed:"));
                    ui.set_next_item_width(150.0);
                    ui.input_float(im_str!("##emulation_speed"), &mut 60.0)
                        .build();

                    ui.separator();

                    ui.text(im_str!("Display scale:"));
                    ui.set_next_item_width(150.0);
                    ui.input_int(im_str!("##display_scale"), &mut display_scale)
                        .build();
                });

            Window::new(im_str!("Disassembly"))
                .position([3.0, 3.0], Condition::FirstUseEver)
                .size([200.0, 467.0], Condition::FirstUseEver)
                .build(&ui, || {
                    ui.checkbox(im_str!("Follow execution"), &mut follow_execution);

                    ChildWindow::new(im_str!("Instruction list")).build(&ui, || {
                        disassembly
                            .iter()
                            .take(0x1000)
                            .for_each(|(addr, instruction)| {
                                Selectable::new(&ImString::new(format!(
                                    "{:#06x}: {}",
                                    addr, instruction
                                )))
                                .selected(&device.cpu().pc == addr)
                                .build(&ui);

                                if follow_execution && &device.cpu().pc == addr {
                                    ui.set_scroll_here_y()
                                }

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
                });

            Window::new(im_str!("Display"))
                .position([375.0, 3.0], Condition::FirstUseEver)
                .always_auto_resize(true)
                .scroll_bar(false)
                .resizable(false)
                .build(&ui, || {
                    Image::new(
                        texture_id,
                        [
                            160.0 * (display_scale as f32),
                            144.0 * (display_scale as f32),
                        ],
                    )
                    .build(&ui);
                });

            let gl_window = display.gl_window();
            let mut target = display.draw();

            target.clear_color_srgb(1.0, 1.0, 1.0, 1.0);

            platform.prepare_render(&ui, gl_window.window());
            let draw_data = ui.render();
            renderer
                .render(&mut target, draw_data)
                .expect("failed to render imgui frame");

            target.finish().expect("failed to finish frame");
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        event => platform.handle_event(imgui.io_mut(), display.gl_window().window(), &event),
    });
}
