use std::fs::File;

use clap::{App, Arg};
use debug::start_debug_view;
use gameboy::{cartridge::Cartridge, device::Device};
use view::start_view;

mod debug;
mod view;

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

    let mut cart = Cartridge::new(
        File::open(
            matches
                .value_of("rom")
                .expect("no rom command line argument supplied"),
        )
        .expect("file not found"),
    )
    .expect("failed to read file");
    cart.try_load();
    let device = Device::new(cart);

    if matches.is_present("debug") {
        start_debug_view(device);
    } else {
        start_view(device);
    }
}
