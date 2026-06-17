/**
 * Fill + shadow atoms used by Shape, Master objects, and base node styling.
 */

import { z } from "zod";

export const fillStyleSchema = z.object({
  color: z.string().optional(),
  transparency: z.number().optional(),
});

export const shadowStyleSchema = z.object({
  type: z.enum(["outer", "inner"]).optional(),
  opacity: z.number().optional(),
  blur: z.number().optional(),
  angle: z.number().optional(),
  offset: z.number().optional(),
  color: z.string().optional(),
});

export type FillStyle = z.infer<typeof fillStyleSchema>;
export type ShadowStyle = z.infer<typeof shadowStyleSchema>;
