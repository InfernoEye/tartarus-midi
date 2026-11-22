# Tartarus MIDI v0.2.0 — Windows release

This is the first **portable Windows x64 build** of Tartarus MIDI: a community prototype that converts the Razer **Tartarus Pro**'s 20 analog keys into MIDI notes.

## What's included

- Note On / Note Off with estimated initial velocity.
- **Polyphonic Aftertouch** (per-note MIDI Poly Key Pressure, status `0xA0`) while holding keys.
- Tartarus D-pad transposition: left/right by one semitone, up/down by one octave.
- Per-key held-note tracking so transpose changes do not strand notes.
- A portable Windows executable, without Rust or Cargo needed on the target PC.

## Getting started

1. Download and extract `tartarus-midi-v0.2.0-windows-x86_64.zip`.
2. Install and run [loopMIDI](https://www.tobias-erichsen.de/software/loopmidi.html), and create a port named **Tartarus MIDI**.
3. Quit **Razer Synapse**, and stop any other Tartarus driver or MIDI bridge.
4. Run `tartarus-midi.exe` from a terminal, or double-click the file.
5. Select **Tartarus MIDI** as the MIDI input in your DAW, enable monitoring and load a synth that supports **polyphonic key pressure**.

The D-pad arrows are **observed, not blocked** from reaching Windows; they may still navigate your DAW. A different loopMIDI port can be selected with the `TARTARUS_MIDI_PORT` environment variable.

## Hardware-tested release

The Windows GitHub Actions build and tests passed. **This project was built for a friend; both the locally compiled V2 and the precompiled v0.2.0 Windows executable from GitHub Releases were confirmed working with a physical Razer Tartarus Pro on Windows 11.** Polyphonic Aftertouch and D-pad semitone/octave transposition were tested on real hardware. This executable is unsigned; Windows SmartScreen may warn before launch. Hardware and firmware compatibility may vary beyond the tested setup. The software is provided as-is, without an installer or warranty. Please report issues and observations in GitHub Issues.

Thanks to [ultramonaka/open-tartarus-driver](https://github.com/ultramonaka/open-tartarus-driver) for the Tartarus Pro HID protocol reverse-engineering. Licensed under **GPL-3.0-or-later**.
