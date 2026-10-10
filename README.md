# Traveller NPC Generator (Rust)

Desktop app that generates Traveller NPCs through a career path and shows them as a character card:
portrait with quote, header (species + rank, career, age, terms, UPP, credits), characteristics with DMs,
skills, career log with muster-out line, and roleplaying trait / wants / hook.

## Build & run

Needs Rust 1.80+ (https://rustup.rs).

```
cargo run --release
```

### Arch Linux / CachyOS

```
# toolchain and the libraries eframe (egui) loads at runtime
sudo pacman -S --needed rust base-devel git libxkbcommon libx11 libxcursor libxi libxrandr wayland mesa vulkan-icd-loader

git clone https://github.com/CharlesPicard/traveller_npc.git
cd traveller_npc
cargo run --release
```

- `./build.sh` builds the release binary; `./build.sh run` builds and launches it.
- Use `rustup` instead of the `rust` package if you prefer it (`sudo pacman -S rustup && rustup default stable`).
- On CachyOS, install your GPU's Vulkan driver if it isn't already there (`vulkan-radeon`, `nvidia-utils` or `vulkan-intel`).
- The binary ends up at `target/release/traveller_npc`; copy it anywhere and run it from the folder where you want exported files to go.
- **Wayland:** winit 0.30 can't receive file drag-and-drop on native Wayland, so the app starts under XWayland
  (it unsets `WAYLAND_DISPLAY` when `DISPLAY` is set). Run `TRAVELLER_NPC_WAYLAND=1 cargo run --release`
  to force native Wayland, where drag-and-drop won't work (type the portrait path instead).
- **Fish shell:** set the variable with `env TRAVELLER_NPC_WAYLAND=1 cargo run --release`.

### Other Linux distros

Debian/Ubuntu may need: `sudo apt install libxkbcommon-dev libgl1-mesa-dev libwayland-dev` (or your distro's equivalents).

Run the generator tests: `cargo test`

## Side panel

- **Generation:** name (blank = random; Aslan get Trokh-style names), species, career, terms (random or 1–7), stat method
  (2D6 / 3D6 drop lowest / heroic), seed + lock (same seed → same NPC), aging checks, career ends on mishap
- **Portrait:** path to a PNG/JPG/WebP, or drag an image onto the window
- **Generate / reroll roleplaying / reroll with same options**
- **Edit current NPC:** name, trait, wants, hook, quote, portrait
- **Roster:** every NPC generated this session; click to view, ✖ to remove
- **Files:** save/load the roster as JSON, export the current NPC as Markdown

## Layout

- `src/model.rs` – dice, careers, species, tables and the generator (std only, unit-tested)
- `src/names.rs` – Aslan name generator built from the Trokh phoneme inventory (given name + clan name)
- `src/ui.rs` – egui side panel and card rendering
- `src/main.rs` – window setup

Add a career by appending to `CAREERS` in `model.rs`; it appears in the dropdown automatically.
