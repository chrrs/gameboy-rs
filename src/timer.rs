use crate::cpu::Interrupts;

pub struct Timer {
    pub divider: u8,
    pub counter: u8,

    pub modulo: u8,
    pub speed: u8,
    pub enabled: bool,

    sub: usize,
    div_sub: usize,
    step_sub: usize,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            divider: 0,
            counter: 0,

            modulo: 0xff,
            speed: 0,
            enabled: false,

            sub: 0,
            div_sub: 0,
            step_sub: 0,
        }
    }

    pub fn cycle(&mut self, cycles: usize) -> Interrupts {
        self.sub += cycles;

        if self.sub >= 4 {
            self.sub -= 4;

            self.div_sub += 1;
            if self.div_sub >= 16 {
                self.div_sub -= 16;
                self.divider = self.divider.wrapping_add(1);
            }

            if self.enabled {
                self.step_sub += 1;
                let div = match self.speed {
                    0b00 => 64,
                    0b01 => 1,
                    0b10 => 4,
                    0b11 => 16,
                    _ => unreachable!(),
                };

                if self.step_sub >= div {
                    self.step_sub -= div;

                    if let Some(i) = self.counter.checked_add(1) {
                        self.counter = i;
                    } else {
                        self.counter = self.modulo;
                        return Interrupts::TIMER;
                    }
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
