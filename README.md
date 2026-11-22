# Tartarus MIDI 🎹

**Turn your Razer Tartarus Pro into a 20-key expressive MIDI controller on Windows 11.**

Read the [Polish documentation](README.pl.md) / [Dokumentacja po polsku](README.pl.md).

> **Experimental v0.2.0 / community toy.** The earlier V1 was reported to emit note-on/note-off messages on the author's device. **This V2 source has not yet been compiled on Windows 11 or validated with physical hardware by the assistant.** Please file a GitHub issue with build errors and device observations. No compiled binary is provided.

## Features

- Reads **20 analog key depths (0–255)** from Tartarus Pro USB HID rather than ordinary keyboard events.
- Sends MIDI **Note On**, **Note Off**, and **per-note Polyphonic Key Pressure / Aftertouch** (`0xA0`).
- Estimates initial **velocity** from how quickly the key reaches the actuation threshold; tracks **pressure** continuously from key depth.
- Transposes new notes using the Tartarus **D-pad**: `←` / `→` = −1 / +1 semitone, `↑` / `↓` = +12 / −12 semitones.
- Uses **Windows Raw Input** to distinguish the Tartarus D-pad from the regular keyboard. No kernel interception driver required for observation.
- Preserves the original MIDI note when a held key receives further aftertouch or Note Off, even if transposition changes mid-note.
- Sends Note Off and MIDI All Notes Off when shut down normally with Ctrl+C.

## What you need

- **Windows 11** (Windows 10 might work, but has not been verified).
- **Razer Tartarus Pro**, USB VID `1532`, PID `0244`. This is **not** the Tartarus V2.
- Rust toolchain from [rustup.rs](https://rustup.rs/) (and MSVC C++ Build Tools if Rust asks for them).
- [loopMIDI](https://www.tobias-erichsen.de/software/loopmidi.html): `midir` does **not** create virtual MIDI output ports on Windows, so loopMIDI hosts the port.
- A DAW, standalone synthesizer, or MIDI monitor. An instrument needs **polyphonic key pressure support** to respond to aftertouch.

## Quick start

1. Quit **Razer Synapse** completely. Stop any other Tartarus application (including `open-tartarus-driver` and V1), which may compete for HID input or inject unwanted keystrokes.
2. Start **loopMIDI** and add a port named `Tartarus MIDI`. Leave loopMIDI running.
3. Open a PowerShell terminal in this project and run:

   ```powershell
   cargo run --release
   ```

4. In your DAW, enable MIDI input from **Tartarus MIDI** on an instrument track and enable input monitoring.
5. Press analog keys. Stop with **Ctrl+C**.

To select a loopMIDI port whose name differs from `Tartarus MIDI`:

```powershell
$env:TARTARUS_MIDI_PORT = "My MIDI Port"
cargo run --release
```

To run source-level unit tests without the physical device (requires a Windows Rust build environment):

```powershell
cargo test
```

### Example console output

The current prototype logs **in Polish**:

```text
klawisz 12 -> nuta  71 ON  vel=126
klawisz 12 -> nuta  71 AT  pre=110
klawisz 12 -> nuta  71 AT  pre= 50
klawisz 12 -> nuta  71 AT  pre= 20
klawisz 12 -> nuta  71 OFF
```

`AT` means Polyphonic Key Pressure (not Channel Pressure). Numbers are examples; actual messages depend on how the key is moved.

## Note mapping and MIDI messages

| Input | MIDI behavior |
| --- | --- |
| Analog keys 01–20 | Base notes **60–79** on MIDI channel **1** |
| Key 09 | Base note **68** |
| Depth at least 100 | Note On (`90 nn velocity`) |
| Depth at most 80 | Note Off (`80 nn 00`) |
| Changing depth while active | Polyphonic Key Pressure (`A0 nn pressure`) |
| D-pad left/right | Transpose **new** notes by −1/+1 semitone |
| D-pad up/down | Transpose **new** notes by +12/−12 semitones |

Pressure is mapped from depth **80–255** to MIDI pressure **0–127**, so low values can occur before Note Off. Pressure messages are sent at most once every **8 ms per key**, and only after a change of at least **2 MIDI steps**. Velocity is estimated from key travel timing, not a hardware force sensor.

**Held-note safety:** if key 09 starts note 68, pressing D-pad up changes the pitch for subsequent notes, but aftertouch and Note Off for the held key still refer to note 68. The transpose range is clamped to **−60..+48** semitones to keep all 20 mapped notes within 0–127.

## Limitations / known issues

- **The D-pad arrows are not suppressed in Windows.** Raw Input observes device-specific events but does not consume them: they can still move the cursor or change selections in your DAW. Device-specific suppression would need a more invasive input interception approach.
- **Aftertouch support depends on your instrument.** Some VSTs support only channel aftertouch, not polyphonic per-note pressure. Map Poly Key Pressure to a musical parameter in the synth if necessary.
- Analog streaming is enabled by sending the Razer **device mode 3** feature report to its control interface. Hardware/firmware behavior may vary. The upstream research notes a reported firmware reconnect issue on some units after this command.
- Raw HID access and D-pad identification are Windows-specific; this is not a generic gaming-keypad MIDI driver.
- This repository currently has **no graphical configuration UI, MPE, installers, or downloadable releases**.
- Graceful shutdown sends Note Off / CC123; abrupt termination, USB unplugging, or power loss may require using your DAW's **Panic / All Notes Off** function.

## Configuration and development

Edit constants in `src/main.rs` to adjust the 20-note `NOTES` array, `NOTE_ON_DEPTH`, `NOTE_OFF_DEPTH`, `PRESSURE_INTERVAL`, and `PRESSURE_MIN_CHANGE`. The D-pad reader is in `src/dpad.rs`.

A Windows CI workflow is included at `.github/workflows/windows.yml` to compile and run unit tests after publication. **A green workflow does not establish that analog HID and device-specific Raw Input work on physical hardware.**

PRs and reports from Tartarus Pro owners are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Credits and license

This project builds on the public HID reverse-engineering notes from [ultramonaka/open-tartarus-driver](https://github.com/ultramonaka/open-tartarus-driver), particularly the analog report layout, device IDs, and device-mode command in its [research notes](https://github.com/ultramonaka/open-tartarus-driver/blob/main/research.md). Thanks to the upstream author for making the investigation public. MIDI I/O uses [midir](https://github.com/Boddlnagg/midir) (MIT).

Licensed under **GPL-3.0-or-later**; see [LICENSE](LICENSE). This is an independent hobby project, not affiliated with or endorsed by Razer.