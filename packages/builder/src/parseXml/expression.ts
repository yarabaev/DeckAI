// ===== Mini expression language for template conditionals/iteration =====
//
// Supports a deliberately small subset, sized to drive <If>/<Choose>/<Foreach>
// without dragging in a full scripting engine:
//
//   identifiers     name, name.member, deeply.nested.path
//   literals        "string"  'string'  123  1.5  true  false  null
//   comparisons     ==  !=  <  <=  >  >=
//   logical         &&  ||  !
//   parens          ( ... )
//   functions       empty(x)   true if undefined / null / "" / [] / {}
//                   not(x)     boolean negation (alias of !x)
//                   length(x)  string / array length, 0 for empty objects
//
// Intentionally NOT supported: arithmetic, regex, indexing[], string concat,
// assignment, ternary. If templates need those they belong in build-time TS,
// not in the markup.

export type Scope = Record<string, unknown>;

type TokenType =
  | "IDENT"
  | "NUMBER"
  | "STRING"
  | "TRUE"
  | "FALSE"
  | "NULL"
  | "EQ"
  | "NEQ"
  | "LT"
  | "LTE"
  | "GT"
  | "GTE"
  | "AND"
  | "OR"
  | "NOT"
  | "LPAREN"
  | "RPAREN"
  | "COMMA"
  | "DOT"
  | "EOF";

interface Token {
  type: TokenType;
  value: string;
  pos: number;
}

type AstNode =
  | { kind: "lit"; value: unknown }
  | { kind: "ident"; path: string[] }
  | { kind: "binop"; op: string; left: AstNode; right: AstNode }
  | { kind: "unop"; op: string; arg: AstNode }
  | { kind: "call"; name: string; args: AstNode[] };

export function evaluateExpression(expr: string, scope: Scope): unknown {
  const tokens = tokenize(expr);
  const parser = new Parser(tokens);
  const ast = parser.parseExpr();
  if (!parser.atEnd()) {
    const t = parser.peek();
    throw new Error(
      `expression "${expr}": unexpected trailing token "${t.value}" at position ${t.pos}`,
    );
  }
  return evalNode(ast, scope);
}

function tokenize(s: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;
  while (i < s.length) {
    const c = s[i]!;
    if (c === " " || c === "\t" || c === "\n" || c === "\r") {
      i++;
      continue;
    }
    if (c >= "0" && c <= "9") {
      let j = i + 1;
      let sawDot = false;
      while (j < s.length) {
        const cc = s[j]!;
        if (cc >= "0" && cc <= "9") {
          j++;
        } else if (cc === "." && !sawDot) {
          sawDot = true;
          j++;
        } else {
          break;
        }
      }
      tokens.push({ type: "NUMBER", value: s.slice(i, j), pos: i });
      i = j;
      continue;
    }
    if (c === '"' || c === "'") {
      const quote = c;
      let j = i + 1;
      let raw = "";
      while (j < s.length && s[j] !== quote) {
        if (s[j] === "\\" && j + 1 < s.length) {
          const next = s[j + 1]!;
          raw += next === "n" ? "\n" : next === "t" ? "\t" : next;
          j += 2;
        } else {
          raw += s[j];
          j++;
        }
      }
      if (j >= s.length) {
        throw new Error(
          `unterminated string literal starting at position ${i}`,
        );
      }
      tokens.push({ type: "STRING", value: raw, pos: i });
      i = j + 1;
      continue;
    }
    if ((c >= "a" && c <= "z") || (c >= "A" && c <= "Z") || c === "_") {
      let j = i + 1;
      while (j < s.length) {
        const cc = s[j]!;
        if (
          (cc >= "a" && cc <= "z") ||
          (cc >= "A" && cc <= "Z") ||
          (cc >= "0" && cc <= "9") ||
          cc === "_"
        ) {
          j++;
        } else {
          break;
        }
      }
      const ident = s.slice(i, j);
      const kw =
        ident === "true"
          ? "TRUE"
          : ident === "false"
            ? "FALSE"
            : ident === "null"
              ? "NULL"
              : null;
      tokens.push({ type: kw ?? "IDENT", value: ident, pos: i });
      i = j;
      continue;
    }
    if (s.startsWith("==", i)) {
      tokens.push({ type: "EQ", value: "==", pos: i });
      i += 2;
      continue;
    }
    if (s.startsWith("!=", i)) {
      tokens.push({ type: "NEQ", value: "!=", pos: i });
      i += 2;
      continue;
    }
    if (s.startsWith("<=", i)) {
      tokens.push({ type: "LTE", value: "<=", pos: i });
      i += 2;
      continue;
    }
    if (s.startsWith(">=", i)) {
      tokens.push({ type: "GTE", value: ">=", pos: i });
      i += 2;
      continue;
    }
    if (s.startsWith("&&", i)) {
      tokens.push({ type: "AND", value: "&&", pos: i });
      i += 2;
      continue;
    }
    if (s.startsWith("||", i)) {
      tokens.push({ type: "OR", value: "||", pos: i });
      i += 2;
      continue;
    }
    if (c === "<") {
      tokens.push({ type: "LT", value: c, pos: i });
      i++;
      continue;
    }
    if (c === ">") {
      tokens.push({ type: "GT", value: c, pos: i });
      i++;
      continue;
    }
    if (c === "!") {
      tokens.push({ type: "NOT", value: c, pos: i });
      i++;
      continue;
    }
    if (c === "(") {
      tokens.push({ type: "LPAREN", value: c, pos: i });
      i++;
      continue;
    }
    if (c === ")") {
      tokens.push({ type: "RPAREN", value: c, pos: i });
      i++;
      continue;
    }
    if (c === ",") {
      tokens.push({ type: "COMMA", value: c, pos: i });
      i++;
      continue;
    }
    if (c === ".") {
      tokens.push({ type: "DOT", value: c, pos: i });
      i++;
      continue;
    }
    throw new Error(`unexpected character "${c}" at position ${i}`);
  }
  tokens.push({ type: "EOF", value: "", pos: s.length });
  return tokens;
}

class Parser {
  private i = 0;
  constructor(private readonly tokens: Token[]) {}

  peek(): Token {
    return this.tokens[this.i]!;
  }
  consume(): Token {
    return this.tokens[this.i++]!;
  }
  atEnd(): boolean {
    return this.peek().type === "EOF";
  }
  expect(type: TokenType): Token {
    const t = this.peek();
    if (t.type !== type) {
      throw new Error(
        `expected ${type} at position ${t.pos}, got ${t.type} "${t.value}"`,
      );
    }
    return this.consume();
  }

  parseExpr(): AstNode {
    return this.parseOr();
  }

  parseOr(): AstNode {
    let left = this.parseAnd();
    while (this.peek().type === "OR") {
      this.consume();
      const right = this.parseAnd();
      left = { kind: "binop", op: "||", left, right };
    }
    return left;
  }

  parseAnd(): AstNode {
    let left = this.parseEq();
    while (this.peek().type === "AND") {
      this.consume();
      const right = this.parseEq();
      left = { kind: "binop", op: "&&", left, right };
    }
    return left;
  }

  parseEq(): AstNode {
    let left = this.parseRel();
    while (this.peek().type === "EQ" || this.peek().type === "NEQ") {
      const op = this.consume().value;
      const right = this.parseRel();
      left = { kind: "binop", op, left, right };
    }
    return left;
  }

  parseRel(): AstNode {
    let left = this.parseUnary();
    while (
      this.peek().type === "LT" ||
      this.peek().type === "LTE" ||
      this.peek().type === "GT" ||
      this.peek().type === "GTE"
    ) {
      const op = this.consume().value;
      const right = this.parseUnary();
      left = { kind: "binop", op, left, right };
    }
    return left;
  }

  parseUnary(): AstNode {
    if (this.peek().type === "NOT") {
      this.consume();
      return { kind: "unop", op: "!", arg: this.parseUnary() };
    }
    return this.parseAtom();
  }

  parseAtom(): AstNode {
    const t = this.peek();
    if (t.type === "NUMBER") {
      this.consume();
      return { kind: "lit", value: parseFloat(t.value) };
    }
    if (t.type === "STRING") {
      this.consume();
      return { kind: "lit", value: t.value };
    }
    if (t.type === "TRUE") {
      this.consume();
      return { kind: "lit", value: true };
    }
    if (t.type === "FALSE") {
      this.consume();
      return { kind: "lit", value: false };
    }
    if (t.type === "NULL") {
      this.consume();
      return { kind: "lit", value: null };
    }
    if (t.type === "LPAREN") {
      this.consume();
      const e = this.parseExpr();
      this.expect("RPAREN");
      return e;
    }
    if (t.type === "IDENT") {
      this.consume();
      if (this.peek().type === "LPAREN") {
        this.consume();
        const args: AstNode[] = [];
        if (this.peek().type !== "RPAREN") {
          args.push(this.parseExpr());
          while (this.peek().type === "COMMA") {
            this.consume();
            args.push(this.parseExpr());
          }
        }
        this.expect("RPAREN");
        return { kind: "call", name: t.value, args };
      }
      const path = [t.value];
      while (this.peek().type === "DOT") {
        this.consume();
        const next = this.expect("IDENT");
        path.push(next.value);
      }
      return { kind: "ident", path };
    }
    throw new Error(
      `unexpected token ${t.type} "${t.value}" at position ${t.pos}`,
    );
  }
}

function evalNode(node: AstNode, scope: Scope): unknown {
  switch (node.kind) {
    case "lit":
      return node.value;
    case "ident":
      return resolvePath(scope, node.path);
    case "unop": {
      const v = evalNode(node.arg, scope);
      if (node.op === "!") return !truthy(v);
      throw new Error(`unsupported unary operator "${node.op}"`);
    }
    case "binop": {
      // Short-circuit logical operators.
      if (node.op === "&&") {
        return truthy(evalNode(node.left, scope))
          ? truthy(evalNode(node.right, scope))
          : false;
      }
      if (node.op === "||") {
        return truthy(evalNode(node.left, scope))
          ? true
          : truthy(evalNode(node.right, scope));
      }
      const l = evalNode(node.left, scope);
      const r = evalNode(node.right, scope);
      switch (node.op) {
        case "==":
          return looseEq(l, r);
        case "!=":
          return !looseEq(l, r);
        case "<":
          return compare(l, r) < 0;
        case "<=":
          return compare(l, r) <= 0;
        case ">":
          return compare(l, r) > 0;
        case ">=":
          return compare(l, r) >= 0;
        default:
          throw new Error(`unsupported binary operator "${node.op}"`);
      }
    }
    case "call": {
      const args = node.args.map((a) => evalNode(a, scope));
      switch (node.name) {
        case "empty":
          return isEmpty(args[0]);
        case "not":
          return !truthy(args[0]);
        case "length":
          return getLength(args[0]);
        default:
          throw new Error(`unknown function "${node.name}()"`);
      }
    }
  }
}

export function resolvePath(scope: Scope, path: string[]): unknown {
  let cur: unknown = scope[path[0]!];
  for (let i = 1; i < path.length; i++) {
    if (cur === null || cur === undefined) return undefined;
    if (typeof cur !== "object") return undefined;
    cur = (cur as Record<string, unknown>)[path[i]!];
  }
  return cur;
}

function truthy(v: unknown): boolean {
  if (v === null || v === undefined) return false;
  if (typeof v === "boolean") return v;
  if (typeof v === "number") return v !== 0 && !Number.isNaN(v);
  if (typeof v === "string") return v.length > 0;
  if (Array.isArray(v)) return v.length > 0;
  return true;
}

function isEmpty(v: unknown): boolean {
  if (v === null || v === undefined) return true;
  if (typeof v === "string") return v.length === 0;
  if (Array.isArray(v)) return v.length === 0;
  if (typeof v === "object") return Object.keys(v).length === 0;
  return false;
}

function getLength(v: unknown): number {
  if (typeof v === "string") return v.length;
  if (Array.isArray(v)) return v.length;
  if (v && typeof v === "object") return Object.keys(v).length;
  return 0;
}

function looseEq(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  // Coerce string<->number on either side so `{m.size} == 40` works whether
  // size arrives as a JSON number or a substituted string.
  if (typeof a === "number" && typeof b === "string") {
    const bn = parseFloat(b);
    return !Number.isNaN(bn) && a === bn;
  }
  if (typeof b === "number" && typeof a === "string") {
    const an = parseFloat(a);
    return !Number.isNaN(an) && b === an;
  }
  return false;
}

function compare(a: unknown, b: unknown): number {
  const an = typeof a === "string" ? parseFloat(a) : a;
  const bn = typeof b === "string" ? parseFloat(b) : b;
  if (
    typeof an === "number" &&
    typeof bn === "number" &&
    !Number.isNaN(an) &&
    !Number.isNaN(bn)
  ) {
    return an - bn;
  }
  if (typeof a === "string" && typeof b === "string") {
    return a < b ? -1 : a > b ? 1 : 0;
  }
  throw new Error(`cannot compare values of type ${typeof a} and ${typeof b}`);
}
