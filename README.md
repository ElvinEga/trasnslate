# TRANSLATE

TRANSLATE is a Rust audio plugin for mix translation checking, built with NIH-plug, egui, and exported as CLAP and VST3.

## Build

Debug check:

```sh
cargo check
```

Optimized library build:

```sh
cargo build --release -p translate
```

Preferred plugin bundle build with the workspace `xtask`:

```sh
cargo run --release --package xtask -- bundle translate --release
```

On Linux, the final link step needs the unversioned `libX11-xcb.so` development library in addition to the runtime `.so.1`. On Debian/Ubuntu systems this is typically provided by `libx11-xcb-dev`.

## Plugin Formats

- CLAP is exported through `nih_export_clap!`
- VST3 is exported through `nih_export_vst3!`
- The package name is `translate`
- The plugin name shown to hosts is `TRANSLATE`

Expected outputs:

- Raw release library on Linux: `target/release/libtranslate.so`
- Bundled plugin outputs from NIH-plug: `target/bundled/`

## Assets

Bundled IR placeholder assets live in `assets/irs/`.

The plugin currently embeds these assets into the binary at compile time. There is no external IR browser or user IR loading path yet.

## Metadata

Current plugin metadata:

- Name: `TRANSLATE`
- Vendor: `Placeholder Vendor`
- Version: `0.1.1`
- CLAP ID: `com.placeholdervendor.translate`
- VST3 class ID: `TranslatePlugin!`

Notes:

- Vendor, support email, and URL are still placeholders and should be replaced before public distribution.
- The VST3 class ID is stable in code and should not be changed after host-facing releases unless you intentionally want hosts to treat it as a different plugin.

## Current Limitations

- Bundled IRs are still placeholder assets, not production captures.
- Loudness Lock is a practical first-pass compensation stage, not LUFS-based matching.
- A/B snapshots are workflow state inside the UI path and are not yet persisted by the host.
- Quick Cycle timing is stable for normal buffers, but not sample-accurate transport sync.
- Release bundling on Linux depends on system GUI/OpenGL/X11 development libraries.

## Packaging

Recommended release flow:

1. Run `cargo fmt`
2. Run `cargo check`
3. Run `cargo build --release -p translate`
4. Run `cargo run --release --package xtask -- bundle translate --release`
5. Inspect `target/bundled/TRANSLATE.clap` and `target/bundled/TRANSLATE.vst3`
6. Copy the resulting `.clap` and `.vst3` artifacts into local host plugin folders for validation

If you want the shorter alias form, this repo also defines:

```sh
cargo xtask bundle translate --release
```

Typical plugin install locations:

- Linux CLAP: `~/.clap/`
- Linux VST3: `~/.vst3/`
- macOS CLAP: `~/Library/Audio/Plug-Ins/CLAP/`
- macOS VST3: `~/Library/Audio/Plug-Ins/VST3/`
- Windows CLAP: `%COMMONPROGRAMFILES%\\CLAP\\`
- Windows VST3: `%COMMONPROGRAMFILES%\\VST3\\`

## Host Validation Checklist

Load and discovery:

- Confirm CLAP build is discovered by a CLAP-capable host
- Confirm VST3 build is discovered by a VST3-capable host
- Confirm plugin metadata shows `TRANSLATE` version `0.1.1`

Editor:

- Open and close the editor repeatedly
- Confirm meters update while audio is playing
- Confirm status text reflects the current preset and IR file

Automation and parameters:

- Automate `Mix`, `Output`, `Mono`, `Bypass`, `Decay`, `Width`, `Low`, `High`
- Confirm automation is smooth and does not produce zipper noise beyond expected parameter ranges
- Confirm `Safety Limiter` and `Loudness Lock` toggle cleanly

State save and restore:

- Save a project with non-default settings and reopen it
- Confirm plugin parameters restore correctly
- Confirm active preset and Quick Cycle settings restore correctly
- Confirm current known limitation: A/B snapshots are not expected to persist yet

Preset and Quick Cycle:

- Step presets manually and confirm click-free switching
- Run Quick Cycle in Manual mode
- Run Quick Cycle in Timed mode
- Confirm `Return to Reference` behavior
- Confirm disabled cycle entries are skipped
- Confirm reordered cycle entries are followed in the new order

CPU and idle behavior:

- Check that idle CPU settles to a reasonable baseline when no editor is open
- Check CPU while audio is passing through with Quick Cycle off
- Check CPU while Quick Cycle is running
- Check repeated open/close of the editor does not cause runaway CPU or leaks

## Recommended Next Steps After MVP

- Replace placeholder metadata with final vendor/support information
- Validate CLAP and VST3 behavior in at least one Linux host and one Windows or macOS host
- Add persisted workflow state for A/B snapshots if that becomes part of the intended workflow
- Revisit the current convolution path before shipping larger IR libraries
