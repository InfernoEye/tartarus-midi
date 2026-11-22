//! Razer Tartarus Pro -> MIDI for Windows 11.
//! Based on the Tartarus Pro HID research by ultramonaka/open-tartarus-driver.
//! D-pad: Windows Raw Input, filtered to VID_1532 / PID_0244 (no extra driver).
//! MIDI: loopMIDI virtual port + midir; notes, polyphonic key pressure.

mod dpad;

use dpad::Direction;
use hidapi::{HidApi, HidDevice};
use midir::{MidiOutput, MidiOutputConnection};
use std::error::Error;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const VID: u16 = 0x1532;
const PID: u16 = 0x0244;
const REPORT_ID: u8 = 0x06;
const KEY_COUNT: usize = 20;

// Physical key 01..20 -> MIDI note. Key 09 = 68.
const NOTES: [u8; KEY_COUNT] = [
    60, 61, 62, 63, 64, 65, 66, 67, 68, 69,
    70, 71, 72, 73, 74, 75, 76, 77, 78, 79,
];

const NOTE_ON_DEPTH: u8 = 100;
const NOTE_OFF_DEPTH: u8 = 80;
const MOTION_DEPTH: u8 = 6;
const PRESSURE_INTERVAL: Duration = Duration::from_millis(8);
const PRESSURE_MIN_CHANGE: u8 = 2;
// Retain the ability to play every mapped note: 60-60=0, 79+48=127.
const MIN_TRANSPOSE: i16 = -60;
const MAX_TRANSPOSE: i16 = 48;

#[derive(Clone, Copy, Default)]
struct KeyState {
    // Exact MIDI note started on this physical key. NEVER derive Note Off/AT
    // from current transpose: the user may change pitch while holding a key.
    active_note: Option<u8>,
    motion_start: Option<Instant>,
    last_pressure: Option<u8>,
    last_pressure_at: Option<Instant>,
}

fn note_with_transpose(base: u8, transpose: i16) -> u8 {
    (i16::from(base) + transpose).clamp(0, 127) as u8
}

fn change_transpose(current: &mut i16, direction: Direction) {
    let delta = match direction {
        Direction::Left => -1,
        Direction::Right => 1,
        Direction::Up => 12,
        Direction::Down => -12,
    };
    let previous = *current;
    *current = (*current + delta).clamp(MIN_TRANSPOSE, MAX_TRANSPOSE);
    if *current != previous {
        let key09 = note_with_transpose(NOTES[8], *current);
        println!(
            "D-pad {:>5} -> transpozycja {:+} półtonów (klawisz 09 = nuta {})",
            direction.name(), *current, key09
        );
    } else {
        println!("D-pad {:>5} -> osiągnięto zakres MIDI 0..127", direction.name());
    }
}

// Pressure is normalized across the *sounding* travel range, rather than
// 0..255. This makes small pressure values possible just before Note Off.
// NOTE_OFF_DEPTH (=80) maps to 0, maximum depth (=255) to 127.
fn depth_to_pressure(depth: u8) -> u8 {
    let travel = u16::from(depth.saturating_sub(NOTE_OFF_DEPTH));
    let span = u16::from(u8::MAX - NOTE_OFF_DEPTH);
    ((travel * 127) / span) as u8
}

fn estimate_velocity(since: Option<Instant>, now: Instant) -> u8 {
    let Some(start) = since else { return 100 };
    let elapsed_ms = now.duration_since(start).as_millis().min(1000) as i32;
    if elapsed_ms < 3 { 100 } else { (150 - 2 * elapsed_ms).clamp(30, 127) as u8 }
}

fn open_midi() -> Result<MidiOutputConnection, Box<dyn Error>> {
    let midi = MidiOutput::new("Tartarus MIDI V2")?;
    let ports = midi.ports();
    let desired = std::env::var("TARTARUS_MIDI_PORT")
        .unwrap_or_else(|_| "Tartarus MIDI".to_owned());
    let selected = ports.iter().find(|port| {
        midi.port_name(port)
            .map(|name| name.to_lowercase().contains(&desired.to_lowercase()))
            .unwrap_or(false)
    });
    let port = match selected {
        Some(port) => port,
        None => {
            eprintln!("Brak portu MIDI zawierającego: {desired:?}");
            for port in &ports {
                eprintln!("  dostępny: {}", midi.port_name(port).unwrap_or_default());
            }
            return Err(io::Error::new(io::ErrorKind::NotFound,
                "Utwórz port 'Tartarus MIDI' w loopMIDI lub ustaw TARTARUS_MIDI_PORT").into());
        }
    };
    println!("MIDI OUT -> {}", midi.port_name(port)?);
    Ok(midi.connect(port, "Tartarus Pro MIDI V2")?)
}

fn open_analog_devices(api: &HidApi) -> Result<Vec<HidDevice>, Box<dyn Error>> {
    let mut devices = Vec::new();
    for info in api.device_list() {
        if info.vendor_id() != VID || info.product_id() != PID { continue; }
        // On Windows, keyboard/mouse boot collections are reserved.
        if info.usage_page() == 0x0001 && matches!(info.usage(), 0x0002 | 0x0006) {
            continue;
        }
        match info.open_device(api) {
            Ok(device) => {
                device.set_blocking_mode(false)?;
                println!("Analogowy interfejs HID: {}", info.interface_number());
                devices.push(device);
            }
            Err(error) => eprintln!("Pominięto HID: {error}"),
        }
    }
    if devices.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound,
            "Brak odczytywalnego interfejsu analogowego Tartarus Pro").into());
    }
    Ok(devices)
}

fn enable_analog_stream(api: &HidApi) -> Result<HidDevice, Box<dyn Error>> {
    let info = api.device_list().find(|info| {
        info.vendor_id() == VID && info.product_id() == PID &&
        info.usage_page() == 0x0001 && info.usage() == 0x0002
    }).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound,
        "Nie znaleziono interfejsu Razer Control"))?;
    let control = info.open_device(api)?;
    // 1-byte report ID + 90-byte Razer feature command (documented upstream).
    let mut command = [0u8; 91];
    command[2] = 0x01; // transaction ID
    command[6] = 0x02; // argument count
    command[7] = 0x00; // command class
    command[8] = 0x04; // set device mode
    command[9] = 0x03; // analog mode
    command[89] = command[3..89].iter().fold(0u8, |crc, byte| crc ^ *byte);
    control.send_feature_report(&command)?;
    println!("Strumień analogowy HID (mode 3) włączony.");
    Ok(control)
}

// Accepts a callback so we can test MIDI logic without real MIDI hardware.
fn process_report<F>(
    depths: &[u8; KEY_COUNT],
    states: &mut [KeyState; KEY_COUNT],
    transpose: i16,
    now: Instant,
    send: &mut F,
) -> Result<(), Box<dyn Error>>
where F: FnMut(&[u8]) -> Result<(), Box<dyn Error>> {
    for (index, state) in states.iter_mut().enumerate() {
        let depth = depths[index];
        if state.active_note.is_none() {
            if depth <= MOTION_DEPTH {
                state.motion_start = None;
                continue;
            }
            if state.motion_start.is_none() { state.motion_start = Some(now); }
            if depth < NOTE_ON_DEPTH { continue; }

            let note = note_with_transpose(NOTES[index], transpose);
            let vel = estimate_velocity(state.motion_start, now);
            send(&[0x90, note, vel])?;
            state.active_note = Some(note);
            state.last_pressure = None;
            state.last_pressure_at = None;
            println!("klawisz {:02} -> nuta {:>3} ON  vel={:>3}", index + 1, note, vel);
        }

        let Some(note) = state.active_note else { continue };
        if depth <= NOTE_OFF_DEPTH {
            send(&[0x80, note, 0])?;
            println!("klawisz {:02} -> nuta {:>3} OFF", index + 1, note);
            *state = KeyState::default();
            continue;
        }

        let pressure = depth_to_pressure(depth);
        let changed = state.last_pressure
            .map(|old| old.abs_diff(pressure) >= PRESSURE_MIN_CHANGE)
            .unwrap_or(true);
        let interval_passed = state.last_pressure_at
            .map(|at| now.duration_since(at) >= PRESSURE_INTERVAL)
            .unwrap_or(true);
        if changed && interval_passed {
            // Channel 1: Polyphonic Key Pressure (MIDI 1.0, status 0xA0).
            send(&[0xA0, note, pressure])?;
            state.last_pressure = Some(pressure);
            state.last_pressure_at = Some(now);
            println!("klawisz {:02} -> nuta {:>3} AT  pre={:>3}", index + 1, note, pressure);
        }
    }
    Ok(())
}

fn release_all(midi: &mut MidiOutputConnection, states: &mut [KeyState; KEY_COUNT]) {
    for state in states.iter_mut() {
        if let Some(note) = state.active_note {
            let _ = midi.send(&[0x80, note, 0]);
        }
        *state = KeyState::default();
    }
    let _ = midi.send(&[0xB0, 123, 0]); // All Notes Off, MIDI channel 1.
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut midi = open_midi()?;
    let api = HidApi::new()?;
    let analog_devices = open_analog_devices(&api)?;
    let _control = enable_analog_stream(&api)?;
    let dpad_rx = dpad::start_listener();

    let keep_running = Arc::new(AtomicBool::new(true));
    let flag = Arc::clone(&keep_running);
    ctrlc::set_handler(move || flag.store(false, Ordering::SeqCst))?;

    println!("Tartarus MIDI V2 aktywny: 20 nut, Poly AT i D-pad (Windows Raw Input).");
    println!("D-pad: LEWO -1, PRAWO +1, GÓRA +12, DÓŁ -12 półtonów.");
    println!("Transpozycja działa dla NOWYCH nut; nuty już trzymane nie zmieniają się.");
    println!("Ctrl+C = zakończenie z Note Off. Strzałki D-pada mogą nadal sterować Windows.");
    let mut transpose: i16 = 0;
    let mut states = [KeyState::default(); KEY_COUNT];
    let mut buf = [0u8; 64];
    let mut received_report = false;

    'poll: while keep_running.load(Ordering::SeqCst) {
        // Apply transposition before new Note On events are processed.
        while let Ok(direction) = dpad_rx.try_recv() {
            change_transpose(&mut transpose, direction);
        }
        for device in &analog_devices {
            match device.read(&mut buf) {
                Ok(size) if size >= 1 + KEY_COUNT && buf[0] == REPORT_ID => {
                    received_report = true;
                    let mut depths = [0u8; KEY_COUNT];
                    depths.copy_from_slice(&buf[1..1 + KEY_COUNT]);
                    let mut send = |message: &[u8]| -> Result<(), Box<dyn Error>> {
                        midi.send(message)?;
                        Ok(())
                    };
                    if let Err(error) = process_report(
                        &depths, &mut states, transpose, Instant::now(), &mut send
                    ) {
                        eprintln!("Błąd MIDI: {error}");
                        break 'poll;
                    }
                }
                Ok(_) => (),
                Err(error) => {
                    eprintln!("Błąd HID / rozłączono Tartarus: {error}");
                    break 'poll;
                }
            }
        }
        thread::sleep(Duration::from_millis(1));
    }

    release_all(&mut midi, &mut states);
    if !received_report {
        eprintln!("Nie odebrano analogowych raportów HID — sprawdź Synapse i USB.");
    }
    println!("Zatrzymano; wysłano Note Off / All Notes Off.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key09_transpose_as_requested() {
        assert_eq!(note_with_transpose(NOTES[8], -1), 67);
        assert_eq!(note_with_transpose(NOTES[8], 0), 68);
        assert_eq!(note_with_transpose(NOTES[8], 1), 69);
        assert_eq!(note_with_transpose(NOTES[8], -12), 56);
        assert_eq!(note_with_transpose(NOTES[8], 12), 80);
    }

    #[test]
    fn pressure_can_be_low_before_key_off() {
        assert_eq!(depth_to_pressure(80), 0);
        assert_eq!(depth_to_pressure(255), 127);
        assert!(depth_to_pressure(108) >= 20);
        assert_eq!(depth_to_pressure(79), 0);
    }

    #[test]
    fn note_off_and_aftertouch_use_original_note_after_transpose() {
        let mut keys = [KeyState::default(); KEY_COUNT];
        let start = Instant::now();
        let mut captured: Vec<Vec<u8>> = Vec::new();
        let mut emit = |data: &[u8]| -> Result<(), Box<dyn Error>> {
            captured.push(data.to_vec()); Ok(())
        };
        let mut depths = [0u8; KEY_COUNT];
        depths[8] = 232; // pressure 110
        process_report(&depths, &mut keys, 0, start, &mut emit).unwrap();
        assert_eq!(keys[8].active_note, Some(68));
        // The D-pad changed the offset while key 09 was held.
        depths[8] = 149; // pressure 50
        process_report(&depths, &mut keys, 12,
            start + Duration::from_millis(12), &mut emit).unwrap();
        depths[8] = 108; // pressure 20, still above Note Off threshold
        process_report(&depths, &mut keys, 12,
            start + Duration::from_millis(24), &mut emit).unwrap();
        depths[8] = 70;
        process_report(&depths, &mut keys, 12,
            start + Duration::from_millis(36), &mut emit).unwrap();
        assert_eq!(&captured[0][..2], &[0x90, 68]);
        assert!(captured.iter().any(|v| v.as_slice() == [0xA0, 68, 110]));
        assert!(captured.iter().any(|v| v.as_slice() == [0xA0, 68, 50]));
        assert!(captured.iter().any(|v| v.as_slice() == [0xA0, 68, 20]));
        assert!(captured.iter().any(|v| v.as_slice() == [0x80, 68, 0]));
        assert!(!captured.iter().any(|v| v.len() > 1 && v[1] == 80));
    }
}