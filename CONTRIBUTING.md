# Contributing

Thanks for helping make this little instrument more useful!

## Good first contributions

- Test on **Windows 11 + Razer Tartarus Pro** and report USB firmware variation.
- Verify the Windows Raw Input D-pad and whether events also reach your DAW.
- Improve velocity and aftertouch curves, MIDI mapping, and calibration.
- Add optional instrumentation / debug logging (avoid collecting personal device paths in public logs).

## Before submitting a PR

1. Run `cargo fmt --check` and `cargo test` on Windows.
2. Prefer small, focused PRs and describe whether behavior was tested on real hardware.
3. Never include private settings, private device paths, firmware dumps, or copyrighted proprietary driver binaries.
4. Preserve the GPL-3.0-or-later license and upstream research attribution.

Please include Windows version, hardware model, firmware revision (if known), and reproduction steps when opening a bug report. Avoid posting your full Raw Input device instance path if it contains personal information.