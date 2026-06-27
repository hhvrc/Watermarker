// Wire types shared with the Rust backend. Keep `Anchor` and `Placement` in sync
// with src-tauri/src/placement.rs.

export type Anchor =
  | 'TopLeft'
  | 'TopCenter'
  | 'TopRight'
  | 'MiddleLeft'
  | 'Center'
  | 'MiddleRight'
  | 'BottomLeft'
  | 'BottomCenter'
  | 'BottomRight';

/** Resolution-independent watermark placement (mirror of Rust `Placement`). */
export interface Placement {
  anchor: Anchor;
  /** Distance from the anchored vertical edge, as a fraction of the shorter side. */
  margin_x_frac: number;
  /** Distance from the anchored horizontal edge, as a fraction of the shorter side. */
  margin_y_frac: number;
  /** Drawn watermark width, as a fraction of image width. */
  width_frac: number;
  /** Rotation about the watermark center, in degrees. */
  rot_deg: number;
  /** Opacity in 0..1. */
  opacity: number;
}

/** Metadata returned by `import_images`. */
export interface ImageMeta {
  id: string;
  name: string;
  width: number;
  height: number;
}

/** Metadata returned by `set_watermark`. */
export interface WatermarkMeta {
  kind: 'svg' | 'raster';
  width: number;
  height: number;
}

/** `set_watermark_persisted` result: metadata + the durable managed copy path. */
export interface WatermarkMetaStored extends WatermarkMeta {
  stored_path: string;
}

/** A named, recallable combination of watermark + placement. */
export interface Preset {
  name: string;
  placement: Placement;
  /** Managed path of the preset's logo copy, or null if it carries no logo. */
  watermark_path: string | null;
}

export type ImageStatus = 'pending' | 'processing' | 'done' | 'error';

/** A source image in the queue, with its own placement (null = inherit sticky default). */
export interface ImageRef {
  id: string;
  name: string;
  width: number;
  height: number;
  placement: Placement | null;
  /** Blob URL of the downscaled, watermark-free preview (lazy-loaded). */
  previewUrl: string | null;
  status: ImageStatus;
  error?: string;
}

export interface WatermarkRef {
  kind: 'svg' | 'raster';
  width: number;
  height: number;
  /** Blob URL of the watermark preview (with alpha). */
  previewUrl: string;
}

export interface OutputSettings {
  dir: string;
  suffix: string;
  overwrite: boolean;
  /** Strip all metadata (EXIF/ICC/etc.) from outputs for privacy. */
  strip_metadata: boolean;
  /** Also write a non-watermarked `<stem>.png` copy (e.g. to convert TIFF inputs). */
  export_clean: boolean;
}

/** Per-item progress event emitted by `process_batch` on `wm://progress`. */
export interface ProgressEvent {
  completed: number;
  total: number;
  image_id: string;
  name: string;
  status: 'done' | 'error';
  error: string | null;
  output_path: string | null;
}

export interface BatchSummary {
  total: number;
  succeeded: number;
  failed: number;
}
