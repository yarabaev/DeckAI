import { describe, it, expect } from "vitest";
import { evaluateExpression, resolvePath } from "./expression.ts";

describe("evaluateExpression", () => {
  describe("literals", () => {
    it("parses numbers, strings, booleans, null", () => {
      expect(evaluateExpression("42", {})).toBe(42);
      expect(evaluateExpression("3.14", {})).toBe(3.14);
      expect(evaluateExpression('"hello"', {})).toBe("hello");
      expect(evaluateExpression("'world'", {})).toBe("world");
      expect(evaluateExpression("true", {})).toBe(true);
      expect(evaluateExpression("false", {})).toBe(false);
      expect(evaluateExpression("null", {})).toBe(null);
    });

    it("handles escapes inside string literals", () => {
      expect(evaluateExpression('"a\\"b"', {})).toBe('a"b');
      expect(evaluateExpression("'a\\nb'", {})).toBe("a\nb");
    });
  });

  describe("identifiers and dotted paths", () => {
    it("looks up scope identifiers", () => {
      expect(evaluateExpression("name", { name: "Alice" })).toBe("Alice");
      expect(evaluateExpression("count", { count: 7 })).toBe(7);
    });

    it("walks dotted paths", () => {
      const scope = {
        m: { tone: { shade: "coral" }, q: "Q1" },
      };
      expect(evaluateExpression("m.q", scope)).toBe("Q1");
      expect(evaluateExpression("m.tone.shade", scope)).toBe("coral");
    });

    it("returns undefined when traversing past null/undefined", () => {
      expect(evaluateExpression("m.missing", { m: {} })).toBe(undefined);
      expect(evaluateExpression("m.tone.shade", { m: { tone: null } })).toBe(
        undefined,
      );
    });
  });

  describe("comparisons", () => {
    it("supports == and !=", () => {
      expect(evaluateExpression("'a' == 'a'", {})).toBe(true);
      expect(evaluateExpression("'a' != 'b'", {})).toBe(true);
      expect(evaluateExpression("1 == 2", {})).toBe(false);
    });

    it("coerces string<->number across ==", () => {
      expect(evaluateExpression("size == 40", { size: "40" })).toBe(true);
      expect(evaluateExpression("size != 40", { size: "41" })).toBe(true);
    });

    it("supports ordering comparisons", () => {
      expect(evaluateExpression("3 < 5", {})).toBe(true);
      expect(evaluateExpression("3 <= 3", {})).toBe(true);
      expect(evaluateExpression("5 > 3", {})).toBe(true);
      expect(evaluateExpression("5 >= 5", {})).toBe(true);
      expect(evaluateExpression("'a' < 'b'", {})).toBe(true);
    });
  });

  describe("logical operators", () => {
    it("evaluates &&, ||, !", () => {
      expect(evaluateExpression("true && false", {})).toBe(false);
      expect(evaluateExpression("true && true", {})).toBe(true);
      expect(evaluateExpression("false || true", {})).toBe(true);
      expect(evaluateExpression("!true", {})).toBe(false);
      expect(evaluateExpression("!(1 == 2)", {})).toBe(true);
    });

    it("treats absent identifiers as falsy", () => {
      expect(evaluateExpression("missing", {})).toBe(undefined);
      expect(evaluateExpression("!missing", {})).toBe(true);
      expect(evaluateExpression("missing || 'fallback'", {})).toBe(true);
    });

    it("short-circuits", () => {
      const scope: Record<string, unknown> = { x: null };
      expect(evaluateExpression("x && x.deep", scope)).toBe(false);
      expect(evaluateExpression("x || 'ok'", scope)).toBe(true);
    });
  });

  describe("functions", () => {
    it("empty() detects null/undefined/empty containers", () => {
      expect(evaluateExpression("empty(x)", { x: null })).toBe(true);
      expect(evaluateExpression("empty(x)", {})).toBe(true);
      expect(evaluateExpression("empty(x)", { x: "" })).toBe(true);
      expect(evaluateExpression("empty(x)", { x: [] })).toBe(true);
      expect(evaluateExpression("empty(x)", { x: {} })).toBe(true);
      expect(evaluateExpression("empty(x)", { x: "a" })).toBe(false);
      expect(evaluateExpression("empty(x)", { x: [1] })).toBe(false);
    });

    it("not() and length() work", () => {
      expect(evaluateExpression("not(true)", {})).toBe(false);
      expect(evaluateExpression("not(false)", {})).toBe(true);
      expect(evaluateExpression("length(xs)", { xs: [1, 2, 3] })).toBe(3);
      expect(evaluateExpression("length(s)", { s: "abc" })).toBe(3);
    });

    it("rejects unknown function names", () => {
      expect(() => evaluateExpression("foo(x)", {})).toThrow(
        /unknown function/,
      );
    });
  });

  describe("error handling", () => {
    it("throws on syntax errors", () => {
      expect(() => evaluateExpression("1 +", {})).toThrow();
      expect(() => evaluateExpression("(1", {})).toThrow();
      expect(() => evaluateExpression('"unterm', {})).toThrow();
    });

    it("rejects unknown characters", () => {
      expect(() => evaluateExpression("1 @ 2", {})).toThrow(
        /unexpected character/,
      );
    });
  });
});

describe("resolvePath", () => {
  it("walks deep paths and short-circuits on null", () => {
    expect(resolvePath({ a: { b: { c: 7 } } }, ["a", "b", "c"])).toBe(7);
    expect(resolvePath({ a: null }, ["a", "b", "c"])).toBe(undefined);
    expect(resolvePath({}, ["a"])).toBe(undefined);
  });
});
