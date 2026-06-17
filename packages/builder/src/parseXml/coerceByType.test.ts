import { describe, it, expect } from "vitest";
import {
  coerceByType,
  coerceBySpec,
  coerceFallback,
  getObjectShape,
  resolveMixedNotationForSpec,
} from "./coerceByType.ts";

describe("coerceByType", () => {
  describe("primitive types", () => {
    it("number: parses integers and floats", () => {
      expect(coerceByType("42", "number")).toEqual({ value: 42, error: null });
      expect(coerceByType("3.14", "number")).toEqual({
        value: 3.14,
        error: null,
      });
      expect(coerceByType("-5", "number")).toEqual({ value: -5, error: null });
    });

    it("number: rejects non-numeric and empty string", () => {
      expect(coerceByType("abc", "number").error).toContain("Cannot convert");
      expect(coerceByType("", "number").error).toContain("Cannot convert");
    });

    it("boolean: only 'true' and 'false' succeed", () => {
      expect(coerceByType("true", "boolean")).toEqual({
        value: true,
        error: null,
      });
      expect(coerceByType("false", "boolean")).toEqual({
        value: false,
        error: null,
      });
      expect(coerceByType("yes", "boolean").error).toContain("Cannot convert");
    });

    it("string: passes through verbatim", () => {
      expect(coerceByType("hello", "string")).toEqual({
        value: "hello",
        error: null,
      });
      expect(coerceByType("", "string")).toEqual({ value: "", error: null });
    });

    it("json: parses arrays and objects", () => {
      expect(coerceByType("[1,2,3]", "json")).toEqual({
        value: [1, 2, 3],
        error: null,
      });
      expect(coerceByType('{"a":1}', "json")).toEqual({
        value: { a: 1 },
        error: null,
      });
    });

    it("json: errors on malformed input", () => {
      expect(coerceByType("not json", "json").error).toContain("Cannot parse");
    });
  });

  describe("length", () => {
    it("accepts numbers, percent strings, and 'max'", () => {
      expect(coerceByType("100", "length")).toEqual({
        value: 100,
        error: null,
      });
      expect(coerceByType("-50", "length")).toEqual({
        value: -50,
        error: null,
      });
      expect(coerceByType("3.14", "length")).toEqual({
        value: 3.14,
        error: null,
      });
      expect(coerceByType("max", "length")).toEqual({
        value: "max",
        error: null,
      });
      expect(coerceByType("50%", "length")).toEqual({
        value: "50%",
        error: null,
      });
    });

    it("rejects malformed length strings", () => {
      expect(coerceByType("abc", "length").error).toContain("Cannot convert");
      expect(coerceByType("50px", "length").error).toContain("Cannot convert");
    });
  });

  describe("color", () => {
    it("strips optional '#' prefix from 6-digit hex", () => {
      expect(coerceByType("#FF0000", "color")).toEqual({
        value: "FF0000",
        error: null,
      });
      expect(coerceByType("00ff00", "color")).toEqual({
        value: "00ff00",
        error: null,
      });
    });

    it("rejects non-hex or wrong-length values", () => {
      expect(coerceByType("red", "color").error).toContain("Cannot convert");
      expect(coerceByType("#ABC", "color").error).toContain("Cannot convert");
    });

    it("iconColor mirrors color behavior", () => {
      expect(coerceByType("#aabbcc", "iconColor")).toEqual({
        value: "aabbcc",
        error: null,
      });
    });
  });

  describe("structured object types", () => {
    it("border: validates color/width/dashType sub-fields", () => {
      const r = coerceByType(
        '{"color":"FF0000","width":2,"dashType":"dash"}',
        "border",
      );
      expect(r.error).toBeNull();
      expect(r.value).toEqual({ color: "FF0000", width: 2, dashType: "dash" });
    });

    it("border: invalid sub-color reports the field", () => {
      const r = coerceByType('{"color":"red","width":1}', "border");
      expect(r.error).toContain('Field "color"');
    });

    it("fill: validates color sub-field", () => {
      expect(coerceByType('{"color":"AABBCC"}', "fill")).toEqual({
        value: { color: "AABBCC" },
        error: null,
      });
    });

    it("shadow: rejects bad type for sub-number field", () => {
      const r = coerceByType(
        '{"type":"outer","opacity":"abc","blur":4}',
        "shadow",
      );
      expect(r.error).toContain("opacity");
    });

    it("imageSizing: round-trips fully-typed JSON", () => {
      const r = coerceByType('{"type":"cover","w":100,"h":80}', "imageSizing");
      expect(r.value).toEqual({ type: "cover", w: 100, h: 80 });
    });
  });

  describe("padding union", () => {
    it("number form is preserved", () => {
      expect(coerceByType("12", "padding")).toEqual({
        value: 12,
        error: null,
      });
    });

    it("object form is parsed", () => {
      const r = coerceByType(
        '{"top":1,"right":2,"bottom":3,"left":4}',
        "padding",
      );
      expect(r.value).toEqual({ top: 1, right: 2, bottom: 3, left: 4 });
    });

    it("invalid garbage errors", () => {
      expect(coerceByType("abc", "padding").error).toContain("no union option");
    });
  });

  describe("boolean-or-object unions (underline / lineArrow)", () => {
    it("underline: 'true'/'false' collapse to boolean", () => {
      expect(coerceByType("true", "underline")).toEqual({
        value: true,
        error: null,
      });
      expect(coerceByType("false", "underline")).toEqual({
        value: false,
        error: null,
      });
    });

    it("underline: object form validates style + color", () => {
      const r = coerceByType('{"style":"sng","color":"FF00FF"}', "underline");
      expect(r.value).toEqual({ style: "sng", color: "FF00FF" });
    });

    it("underline: bare string rejects with union message", () => {
      expect(coerceByType("solid", "underline").error).toContain(
        "no union option",
      );
    });

    it("lineArrow: same union semantics as underline", () => {
      expect(coerceByType("true", "lineArrow").value).toBe(true);
      expect(
        coerceByType('{"type":"triangle","color":"AAAAAA"}', "lineArrow"),
      ).toEqual({ value: { type: "triangle", color: "AAAAAA" }, error: null });
    });
  });

  describe("json with explicit objectShape (backgroundImage / connectorStyle)", () => {
    it("validates declared sub-fields", () => {
      const r = coerceByType('{"src":"bg.png","sizing":"cover"}', "json", {
        src: "string",
        sizing: "string",
      });
      expect(r.value).toEqual({ src: "bg.png", sizing: "cover" });
    });

    it("falls back to plain JSON when no shape supplied", () => {
      const r = coerceByType('{"x":1}', "json");
      expect(r.value).toEqual({ x: 1 });
    });
  });

  describe("getObjectShape", () => {
    it("returns implied shape for typed coerces", () => {
      expect(getObjectShape({ coerce: "padding" })).toEqual({
        top: "number",
        right: "number",
        bottom: "number",
        left: "number",
      });
      expect(getObjectShape({ coerce: "border" })?.color).toBe("color");
    });

    it("returns explicit objectShape for json types", () => {
      expect(
        getObjectShape({
          coerce: "json",
          objectShape: { src: "string", sizing: "string" },
        }),
      ).toEqual({ src: "string", sizing: "string" });
    });

    it("returns undefined for plain string types", () => {
      expect(getObjectShape({ coerce: "string" })).toBeUndefined();
      expect(getObjectShape({ coerce: "number" })).toBeUndefined();
    });
  });

  describe("coerceFallback", () => {
    it("recognizes booleans, numbers, JSON; otherwise returns string", () => {
      expect(coerceFallback("true")).toBe(true);
      expect(coerceFallback("false")).toBe(false);
      expect(coerceFallback("42")).toBe(42);
      expect(coerceFallback('{"a":1}')).toEqual({ a: 1 });
      expect(coerceFallback("hello")).toBe("hello");
    });
  });

  describe("resolveMixedNotationForSpec", () => {
    it("merge: numeric padding shorthand expands to box", () => {
      const r = resolveMixedNotationForSpec("8", { coerce: "padding" });
      expect(r).toEqual({
        mode: "merge",
        value: { top: 8, right: 8, bottom: 8, left: 8 },
      });
    });

    it("merge: object shorthand merges under dot fields", () => {
      const r = resolveMixedNotationForSpec('{"color":"FF0000","width":1}', {
        coerce: "border",
      });
      expect(r).toEqual({
        mode: "merge",
        value: { color: "FF0000", width: 1 },
      });
    });

    it("ignore: boolean shorthand on underline/lineArrow yields ignore", () => {
      expect(
        resolveMixedNotationForSpec("true", { coerce: "underline" }),
      ).toEqual({ mode: "ignore" });
      expect(
        resolveMixedNotationForSpec("false", { coerce: "lineArrow" }),
      ).toEqual({ mode: "ignore" });
    });

    it("conflict: non-mergeable scalar on a structured spec", () => {
      const r = resolveMixedNotationForSpec("garbage", { coerce: "border" });
      expect(r).toEqual({ mode: "conflict" });
    });

    it("conflict: spec without object shape", () => {
      const r = resolveMixedNotationForSpec("anything", { coerce: "string" });
      expect(r).toEqual({ mode: "conflict" });
    });
  });

  describe("coerceBySpec", () => {
    it("forwards objectShape from spec", () => {
      expect(
        coerceBySpec('{"src":"x","sizing":"cover"}', {
          coerce: "json",
          objectShape: { src: "string", sizing: "string" },
        }),
      ).toEqual({ value: { src: "x", sizing: "cover" }, error: null });
    });
  });
});
