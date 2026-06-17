/**
 * Underline + default text style atoms.
 */

import { z } from "zod";

export const underlineStyleSchema = z.enum([
  "dash",
  "dashHeavy",
  "dashLong",
  "dashLongHeavy",
  "dbl",
  "dotDash",
  "dotDotDash",
  "dotted",
  "dottedHeavy",
  "heavy",
  "none",
  "sng",
  "wavy",
  "wavyDbl",
  "wavyHeavy",
]);

export const underlineSchema = z.union([
  z.boolean(),
  z.object({
    style: underlineStyleSchema.optional(),
    color: z.string().optional(),
  }),
]);

export const defaultTextStyleSchema = z.object({
  fontFamily: z.string().optional(),
  fontSize: z.number().optional(),
  color: z.string().optional(),
  bold: z.boolean().optional(),
  italic: z.boolean().optional(),
  underline: underlineSchema.optional(),
  strike: z.boolean().optional(),
  highlight: z.string().optional(),
  lineHeight: z.number().optional(),
});

export type UnderlineStyle = z.infer<typeof underlineStyleSchema>;
export type Underline = z.infer<typeof underlineSchema>;
export type DefaultTextStyle = z.infer<typeof defaultTextStyleSchema>;
