# Watermarker

A fast, batch image watermarking desktop app. Drop in your images, position a watermark once, review every result, and save the whole set in one pass.

Built with [Tauri](https://tauri.app/) (Rust core) + [SvelteKit](https://kit.svelte.dev/) + TypeScript, by [hhvrc](https://github.com/hhvrc). **Rust is the engine and the frontend is just the interface:** all pixel work (decode, downscale, composite, encode, write) happens in Rust, so the UI never touches full-resolution bitmaps.

## Features

- **Batch watermarking.** Import many images and export them all in one commit.
- **Per-image placement.** Position the watermark independently on each image, with the last edit carried forward as a sticky default.
- **WYSIWYG previews.** One shared Rust `composite()` drives both the on-screen preview and the export, so what you see is exactly what you get. Live dragging uses Canvas2D for instant feedback, and the review grid shows the exact Rust-rendered output.
- **Resolution-independent placement.** An anchor (9-grid) plus relative margins and size, so the same settings look right on any image dimension. Free-drag snaps to the nearest anchor.
- **Interactive editor.** Drag to move, corner handles to resize (aspect-preserving), plus Position / Margin / Size / Opacity controls.
- **SVG & raster watermarks.** SVG logos are rasterized crisply at the target size (resvg/usvg/tiny-skia), and transparent borders are auto-trimmed so the watermark box hugs its content.
- **Presets.** Save and recall named bundles of watermark + full placement.
- **Clean export.** Optionally also write a non-watermarked PNG of the source, handy for converting TIFF or other inputs.
- **Metadata handling.** Preserves EXIF/ICC into the output, bakes EXIF orientation into the pixels, and records a standard PNG `Software` tag identifying the editor. A privacy option strips all metadata (including the Software tag) so outputs carry none.
- **Persistent settings.** Watermark, output folder, placement, and presets are remembered across launches (Rust-backed `settings.json`, not localStorage).

### Formats

- **Input:** PNG, JPG, TIFF, WebP
- **Output:** PNG
- **Watermarks:** SVG (vector) or raster

## How it works

1. **Load.** Drag images onto the window (native drag-drop hands Rust file paths) or use the picker.
2. **Position.** Set the watermark placement per image in the editor, nudging each one as needed.
3. **Stage.** Open the review grid showing every image with its exact Rust-rendered result.
4. **Commit.** Save the whole set to your output folder.

Dropping a watermark file onto the watermark zone sets it (and persists it); dropping elsewhere imports images.

## Getting started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- Node.js `^26` and [pnpm](https://pnpm.io/) `11.9+`
- Platform [dependencies for Tauri](https://tauri.app/start/prerequisites/)

### Install

```sh
pnpm install
```

### Develop

```sh
pnpm tauri dev
```

### Build

```sh
pnpm tauri build
```

## Project layout

```
src/                      SvelteKit frontend (the interface)
  lib/components/         Editor, queue, review grid, presets, controls
  lib/ipc.ts              Typed wrappers over Tauri commands
  lib/placement*.ts       Placement math + canvas mapping (unit-tested)
src-tauri/src/            Rust engine
  decode.rs               Image decoding (with decompression-bomb limits)
  watermark.rs            Watermark loading, SVG rasterization, trim
  placement.rs            Anchor + relative-size placement resolution
  compositor.rs           Blending / rotation / scaling
  preview.rs              Downscaled previews for display
  export.rs               PNG encoding + write
  metadata.rs             EXIF/ICC transplant + Software tag
  settings.rs             Persistent settings & managed watermark store
  commands.rs             Tauri command surface
```

## Development

```sh
# Frontend
pnpm check            # svelte-check (keep at 0 warnings)
pnpm test             # vitest unit tests
pnpm format           # prettier

# Rust (from src-tauri/)
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --release
```

CI runs `pnpm check` + `pnpm test` (frontend) and `cargo fmt --check` / `clippy -D warnings` / `cargo test` (Rust). Tagging `v*` builds a draft GitHub release.

## License

[GNU General Public License v3.0 or later](LICENSE).
