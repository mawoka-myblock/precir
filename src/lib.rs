#![no_std]
extern crate alloc;
use alloc::vec::Vec;
use esp_hal::{gpio::Level, rmt::PulseCode};
pub mod commands;
// return µS
pub fn pp16_symbol_duration(symbol: u8) -> u16 {
    match symbol & 0x0F {
        // mask to ensure only 4 bits
        0x0 => 27,  // 0000
        0x1 => 51,  // 0001
        0x2 => 35,  // 0010
        0x3 => 43,  // 0011
        0x4 => 147, // 0100
        0x5 => 123, // 0101
        0x6 => 139, // 0110
        0x7 => 131, // 0111
        0x8 => 83,  // 1000
        0x9 => 59,  // 1001
        0xA => 75,  // 1010
        0xB => 67,  // 1011
        0xC => 91,  // 1100
        0xD => 115, // 1101
        0xE => 99,  // 1110
        0xF => 107, // 1111
        _ => 0,     // unreachable, but Rust wants a fallback
    }
}

pub fn frame_to_pulses(frame: Vec<u8>) -> Vec<u32> {
    let mut pulses: Vec<u32> = Vec::new();
    for byte in frame {
        let mut nibble = byte;
        for _ in 0..2 {
            // two 4-bit symbols per byte
            let symbol = nibble & 0x0F; // extract LSB nibble
            pulses.push(PulseCode::new(
                Level::High,
                21 * 80,
                Level::Low,
                (pp16_symbol_duration(symbol) - 21) * 80,
            ));
            nibble >>= 4; // next symbol
        }
    }
    pulses.push(PulseCode::empty());
    pulses
}
