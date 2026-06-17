/**
 * Build-time option types shared by `buildPptx` and `buildContext`.
 *
 * Lives in its own module so that `buildContext.ts` can import the
 * option shapes without pulling in the full `buildPptx.ts` runtime —
 * removing a circular file-level dependency.
 */

export interface ImageSrcGuardOptions {
  /** URL schemes allowed for <Image src> and <Master backgroundPath>. */
  allowSchemes?: string[];
  /**
   * If set, file:// and relative paths must resolve under this directory.
   * Paths outside the base dir emit INVALID_IMAGE_SRC and are dropped.
   */
  allowBaseDir?: string;
}

export interface MasterPptxLimits {
  /** Maximum total size of the masterPptx buffer in bytes. Default 50 MB. */
  maxBytes?: number;
  /** Maximum size of a single extracted image in bytes. Default 5 MB. */
  maxImageBytes?: number;
}
