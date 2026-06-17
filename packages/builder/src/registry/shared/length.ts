/**
 * Length atom: number, "max", or `\d+%` percentage string.
 *
 * Used as the canonical type for `w`, `h`, and other dimensional
 * attributes that accept either a number, the keyword `"max"`, or a
 * percent string like `"50%"`.
 */

import { z } from "zod";

export const lengthSchema = z.union([
  z.number(),
  z.literal("max"),
  z.string().regex(/^\d+%$/),
]);

export type Length = z.infer<typeof lengthSchema>;
