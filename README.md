# Tartarus MIDI 🎹

**Turn your Razer Tartarus Pro into a 20-key expressive MIDI controller on Windows 11.**

Read the [Polish documentation](README.pl.md) / [Dokumentacja po polsku](README.pl.md).

> **Experimental v0.2.0 / community toy.** Windows GitHub Actions **builds and tests pass**, and a [portable Windows x64 ZIP is available](https://github.com/InfernoEye/tartarus-midi/releases/tag/v0.2.0). **Physical-hardware behavior of V2 has not yet been verified.** Please open a GitHub issue with your observations.

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
- Rust toolchain from [rustup.rs](https://rustup.rs/) **only if building from source** (and MSVC C++ Build Tools if Rust asks for them).
- [loopMIDI](https://www.tobias-erichsen.de/software/loopmidi.html): `midir` does **not** create virtual MIDI output ports on Windows, so loopMIDI hosts the port.
- A DAW, standalone synthesizer, or MIDI monitor. An instrument needs **polyphonic key pressure support** to respond to aftertouch.

## Quick start (Windows x64 release)

1. Download the [**v0.2.0 Windows ZIP**](https://github.com/InfernoEye/tartarus-midi/releases/download/v0.2.0/tartarus-midi-v0.2.0-windows-x86_64.zip) and extract it. You do **not** need Rust to run the released executable.
2. Quit **Razer Synapse** and stop other Tartarus applications (including `open-tartarus-driver` and V1).
3. Start **loopMIDI**, create a port named `Tartarus MIDI`, and leave it running.
4. Start `tartarus-midi.exe` (preferably from PowerShell to see MIDI event logs).
5. In your DAW, enable **Tartarus MIDI** as an input, load an instrument, and enable MIDI monitoring. Stop with **Ctrl+C**.

To use another loopMIDI port name, set `TARTARUS_MIDI_PORT` before launching the executable.

### Building from source

Install Rust and MSVC Build Tools as needed, then run:

```powershell
cargo run --release
```

Run the unit tests without physical hardware using `cargo test`.

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
- This repository currently has **no graphical configuration UI, MPE, or installer**; the release is a portable, unsigned executable.
- Graceful shutdown sends Note Off / CC123; abrupt termination, USB unplugging, or power loss may require using your DAW's **Panic / All Notes Off** function.

## Configuration and development

Edit constants in `src/main.rs` to adjust the 20-note `NOTES` array, `NOTE_ON_DEPTH`, `NOTE_OFF_DEPTH`, `PRESSURE_INTERVAL`, and `PRESSURE_MIN_CHANGE`. The D-pad reader is in `src/dpad.rs`.

The [Windows CI workflow](.github/workflows/windows.yml) runs tests and builds the project. The [release workflow](.github/workflows/release.yml) packages a Windows x64 ZIP and publishes a GitHub pre-release via Actions. For future versions, update `Cargo.toml` and `RELEASE_NOTES.md`, then use **Actions → Publish Windows release → Run workflow** with a matching `vX.Y.Z` tag. **Passing CI does not prove physical-hardware compatibility.**

PRs and reports from Tartarus Pro owners are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Credits and license

This project builds on the public HID reverse-engineering notes from [ultramonaka/open-tartarus-driver](https://github.com/ultramonaka/open-tartarus-driver), particularly the analog report layout, device IDs, and device-mode command in its [research notes](https://github.com/ultramonaka/open-tartarus-driver/blob/main/research.md). Thanks to the upstream author for making the investigation public. MIDI I/O uses [midir](https://github.com/Boddlnagg/midir) (MIT).

Licensed under **GPL-3.0-or-later**; see [LICENSE](LICENSE). This is an independent hobby project, not affiliated with or endorsed by Razer.