use std::fs::File;

use clap::{App, Arg};
use debug::start_debug_view;
use gameboy::{cartridge::Cartridge, device::Device};

mod debug;

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
    let device = Device::new(cart);

    if matches.is_present("debug") {
        start_debug_view(device);
    }
}
