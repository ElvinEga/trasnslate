Read AGENTS.md first and follow it closely.

We are building TRANSLATE, a Rust audio plugin for mix translation checking.

Tech stack:
- Rust
- NIH-plug
- egui
- CLAP + VST3

Goal for this task:
Create the initial project scaffold and complete Milestone 1 only.

Scope:
1. Create the Rust workspace and plugin crate
2. Set up NIH-plug
3. Set up CLAP and VST3 targets
4. Set up egui editor integration
5. Add a clean folder/module structure for:
   - plugin
   - params
   - ui
   - dsp
   - ir
6. Implement a minimal pass-through plugin
7. Add plugin metadata:
   - name: TRANSLATE
   - vendor: use a placeholder vendor name for now
   - version: 0.1.0
8. Create a very basic egui editor window with placeholder controls for:
   - Preset
   - Decay
   - Mix
   - Width
   - Low
   - High
   - Output
   - Mono
   - Bypass
   - Quick Cycle
9. Make sure the project builds successfully

Important constraints:
- Prioritize correctness and maintainability
- Keep dependencies minimal
- Do not add advanced DSP yet
- Do not add convolution yet
- Do not add custom IR loading yet
- Do not over-engineer the UI
- Keep the code audio-thread safe by default
- No allocations or blocking work in the audio callback
- Keep UI and DSP loosely coupled

Process:
- First inspect the repo and AGENTS.md
- Then make a short implementation plan
- Then implement in small steps
- After coding, run the relevant build/check commands
- Fix any errors you introduced
- At the end, summarize:
  1. what was created
  2. what still remains for Milestone 2
  3. any risks or follow-up notes

Definition of done:
- The plugin project structure exists
- The plugin builds
- The plugin exports CLAP and VST3 targets
- The plugin loads as a pass-through plugin in code
- The egui editor exists with placeholder controls
- The code is organized and ready for DSP implementation next

Do not start Milestone 2.
Stop after Milestone 1 is complete.
