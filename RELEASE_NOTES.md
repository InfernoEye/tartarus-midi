# Tartarus MIDI v0.2.0 — experimental Windows release

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

## Experimental status

This build is automated and checked by Windows GitHub Actions. **The project owner reports that the chat-distributed V2 source compiled successfully and worked with a physical Tartarus Pro on Windows 11.** Polyphonic Aftertouch and D-pad behavior have not yet been individually confirmed, and the published release ZIP has not been separately tested on hardware. This binary is unsigned; Windows SmartScreen may warn before launch. It is provided as-is, without installer or warranties. Please report issues, your device firmware, and observed MIDI output in GitHub Issues.

Thanks to [ultramonaka/open-tartarus-driver](https://github.com/ultramonaka/open-tartarus-driver) for the Tartarus Pro HID protocol reverse-engineering. Licensed under **GPL-3.0-or-later**.
