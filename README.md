# gameboy-rs
A simple non-accurate gameboy emulator made in Rust. It doesn't have sound (yet).

## Building and running
### Running
```bash
$ cargo run -- path/to/rom.gb
```
You can also run the debug view using the `-d` flag:
```bash
$ cargo run -- path/to/rom.gb -d
```

Save games will appear on closing the emulator in the `saves` folder.

## Credits
- The [gameboy pandocs](https://gbdev.io/pandocs/), the best gameboy resource out there.
- [mooneye-gb](https://github.com/Gekkio/mooneye-gb) for some specific implementation details.
- [Eric Haskings explaining how the DAA instruction works](https://ehaskins.com/2018-01-30%20Z80%20DAA/), which I still don't fully understand.
- [Imran Nazar's gameboy in JavaScript tutorial](http://imrannazar.com/GameBoy-Emulation-in-JavaScript) as a general guideline on where to start.
- The [/r/EmuDev reddit community](https://www.reddit.com/r/EmuDev/) and their discord for just general usefulness.
