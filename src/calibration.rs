//! The production interface: how a flow rig gets its verdict into the instrument.
//!
//! A line-based ASCII protocol on `eUSCI_A0`. ASCII rather than something compact because the other
//! end of this is a rig in a calibration room and a person watching a terminal, and a protocol they
//! can read is worth more than the bytes it saves.
//!
//! # It stops existing when the instrument is sealed
//!
//! This is the whole of the answer to WELMEC 7.2 P4, which asks that commands arriving over a
//! communication interface must not be able to influence the instrument inadmissibly. Rather than
//! enumerate which commands are admissible, a sealed instrument does not run this code at all:
//! [`run`] returns immediately, nothing is read from the UART, and the write path in
//! [`params::store`] would refuse anyway.
//!
//! That also settles the energy question. A UART kept listening is milliamps; an instrument in the
//! field never opens one.
//!
//! # The commands
//!
//! | | |
//! | --- | --- |
//! | `I` | identity: software version, image checksum, image length |
//! | `P` | the current parameters |
//! | `T` | the reading, and the write counter |
//! | `M` | run one measurement and report the raw difference in flight time |
//! | `W <n> <value>` | set parameter `n`. Refused once sealed |
//! | `S` | seal the instrument. There is no command to unseal it |
//!
//! `M` is the one the rig actually uses: set a known flow, read the picoseconds the instrument
//! reports, and from a run of those work out the zero offset and the correction that go back in
//! through `W`.

use embassy_msp430::uart::Uart;

use crate::legal::identity;
use crate::legal::params::{self, Params, SEALED};

/// Longest command line accepted.
const LINE: usize = 32;

/// Write a signed decimal, then a space.
async fn num(uart: &mut Uart<'_>, value: i32) {
    let mut digits = [0u8; 12];
    let negative = value < 0;
    let mut v = value.unsigned_abs();
    let mut i = digits.len();
    loop {
        i -= 1;
        digits[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 || i == 1 {
            break;
        }
    }
    if negative {
        i -= 1;
        digits[i] = b'-';
    }
    let _ = uart.write(&digits[i..]).await;
    let _ = uart.write(b" ").await;
}

/// Read one decimal starting at `at`, and say where it ended.
///
/// Returns `None` on anything that is not a number, including one too long to hold — a rig sending
/// nonsense should get a refusal rather than a wrapped value written into a calibration.
fn number(line: &[u8], at: usize) -> Option<(i32, usize)> {
    let mut i = at;
    while i < line.len() && line[i] == b' ' {
        i += 1;
    }
    let negative = i < line.len() && line[i] == b'-';
    if negative {
        i += 1;
    }
    let start = i;
    let mut value: i32 = 0;
    while i < line.len() && line[i].is_ascii_digit() {
        value = value.checked_mul(10)?.checked_add((line[i] - b'0') as i32)?;
        i += 1;
    }
    if i == start {
        return None;
    }
    Some((if negative { -value } else { value }, i))
}

/// Assign parameter `index`. The numbering is the protocol and must not be reordered.
fn set(p: &mut Params, index: i32, value: i32) -> bool {
    match index {
        0 => p.serial = value as u32,
        1 => p.path_um = value as u32,
        2 => p.axial_um = value as u32,
        3 => p.bore_area_mm2_100 = value as u32,
        4 => p.calibration_ppm = value,
        5 => p.zero_offset_ps = value,
        6 => p.excitation_hz = value as u32,
        7 => p.burst_threshold = value as i16,
        _ => return false,
    }
    true
}

/// Serve the production interface until the instrument is sealed.
///
/// Returns as soon as a `S` command succeeds, and returns immediately if the instrument was already
/// sealed — which is the case for every instrument that has left the calibration room.
pub async fn run(uart: &mut Uart<'_>, params: &mut Params) {
    if params.is_calibrated() {
        return;
    }

    let _ = uart.write(b"uflowmeter calibration\r\n").await;

    let mut line = [0u8; LINE];
    let mut len = 0usize;

    loop {
        let mut byte = [0u8; 1];
        if uart.read(&mut byte).await.is_err() {
            return;
        }
        if byte[0] != b'\r' && byte[0] != b'\n' {
            if len < LINE {
                line[len] = byte[0];
                len += 1;
            }
            continue;
        }
        if len == 0 {
            continue;
        }

        let cmd = &line[..len];
        match cmd[0] {
            b'I' => {
                let _ = uart.write(identity::SOFTWARE_VERSION.as_bytes()).await;
                let _ = uart.write(b" ").await;
                // The checksum is reported in two halves, since everything else here is signed.
                let crc = identity::image_crc();
                num(uart, (crc >> 16) as i32).await;
                num(uart, (crc & 0xffff) as i32).await;
                num(uart, identity::image_len() as i32).await;
            }
            b'P' => {
                num(uart, params.serial as i32).await;
                num(uart, params.path_um as i32).await;
                num(uart, params.axial_um as i32).await;
                num(uart, params.bore_area_mm2_100 as i32).await;
                num(uart, params.calibration_ppm).await;
                num(uart, params.zero_offset_ps).await;
                num(uart, params.excitation_hz as i32).await;
                num(uart, params.burst_threshold as i32).await;
                num(uart, params.writes as i32).await;
            }
            b'W' => {
                // Two numbers after the letter: which parameter, and what to set it to. A write
                // that cannot be parsed exactly is refused rather than half-applied -- this is the
                // one path in the firmware that changes what the instrument measures.
                let parsed = number(cmd, 1)
                    .and_then(|(index, next)| number(cmd, next).map(|(value, _)| (index, value)));

                let reply: &[u8] = match parsed {
                    Some((index, value)) => {
                        // Applied to a copy, so a rejected index cannot leave the live parameters
                        // half-written.
                        let mut candidate = *params;
                        if !set(&mut candidate, index, value) {
                            b"ERR"
                        } else {
                            match params::store(&candidate) {
                                Ok(()) => {
                                    // Read back rather than assuming: what the instrument now holds
                                    // is what the rig should see, including the incremented counter.
                                    *params = params::load();
                                    b"OK"
                                }
                                Err(params::WriteError::Sealed) => b"SEALED",
                                Err(params::WriteError::CounterExhausted) => b"EXHAUSTED",
                            }
                        }
                    }
                    None => b"ERR",
                };
                let _ = uart.write(reply).await;
            }
            b'S' => {
                let mut sealed = *params;
                sealed.seal = SEALED;
                match params::store(&sealed) {
                    Ok(()) => {
                        *params = params::load();
                        let _ = uart.write(b"SEALED\r\n").await;
                        // Nothing further is served. The instrument has left the calibration room.
                        return;
                    }
                    Err(_) => {
                        let _ = uart.write(b"ERR").await;
                    }
                }
            }
            _ => {
                let _ = uart.write(b"?").await;
            }
        }
        let _ = uart.write(b"\r\n").await;
        len = 0;
    }
}
