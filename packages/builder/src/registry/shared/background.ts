/**
 * Background-image atom: optional source + sizing keyword.
 */

import { z } from "zod";

const backgroundImageSizingSchema = z.enum(["cover", "contain"]);

export const backgroundImageSchema = z.object({
  src: z.string(),
  sizing: backgroundImageSizingSchema.optional(),
});

export type BackgroundImageSizing = z.infer<typeof backgroundImageSizingSchema>;
export type BackgroundImage = z.infer<typeof backgroundImageSchema>;
