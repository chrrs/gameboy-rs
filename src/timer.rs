use crate::cpu::Interrupts;

pub struct Timer {
    pub divider: u8,
    pub counter: u8,

    pub modulo: u8,
    pub speed: u8,
    pub enabled: bool,

    div_clock: usize,
    counter_clock: usize,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            divider: 0,
            counter: 0,

            modulo: 0xff,
            speed: 0,
            enabled: false,

            div_clock: 0,
            counter_clock: 0,
        }
    }

    pub fn cycle(&mut self, cycles: usize) -> Interrupts {
        self.div_clock += cycles;
        if self.div_clock >= 64 {
            self.div_clock -= 64;
            self.divider = self.divider.wrapping_add(1);
        }

        if self.enabled {
            let period = match self.speed {
                0b00 => 256,
                0b01 => 4,
                0b10 => 16,
                0b11 => 64,
                _ => unreachable!(),
            };

            self.counter_clock += cycles;
            if self.counter_clock >= period {
                self.counter_clock -= period;
                self.counter = self.counter.wrapping_add(1);

                if self.counter == 0 {
                    self.counter = self.modulo;
                    return Interrupts::TIMER;
                }
            }
        }

        Interrupts::empty()
    }

    pub fn timer_control(&self) -> u8 {
        let mut result = self.speed;

        if self.enabled {
            result |= 0b100;
        }

        result
    }

    pub fn set_timer_control(&mut self, value: u8) {
        self.speed = value & 0b11;
        self.enabled = value & 0b100 != 0;
    }
}
