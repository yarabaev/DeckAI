/**
 * Padding atom: a single number or per-side `{top,right,bottom,left}` object.
 * Used for both `padding` and `margin` attributes.
 */

import { z } from "zod";

export const paddingSchema = z.union([
  z.number(),
  z.object({
    top: z.number().optional(),
    right: z.number().optional(),
    bottom: z.number().optional(),
    left: z.number().optional(),
  }),
]);

export type Padding = z.infer<typeof paddingSchema>;
