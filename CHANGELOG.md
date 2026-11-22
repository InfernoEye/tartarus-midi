# Changelog

## 0.2.0 — Windows x64 release (hardware-tested)

- 20-key analog HID → MIDI bridge for Windows 11.
- Note On/Off with estimated velocity and Polyphonic Key Pressure with per-key updates.
- Device-identified D-pad semitone and octave transposition.
- Windows Raw Input listener; no kernel-level D-pad suppression.
- Portable Windows x64 executable distributed through GitHub Releases; GitHub Actions builds and tests.

Built for a friend and tested on a physical Razer Tartarus Pro with Windows 11. Both the locally compiled program and the precompiled release executable worked; Polyphonic Aftertouch and D-pad transposition were also tested.