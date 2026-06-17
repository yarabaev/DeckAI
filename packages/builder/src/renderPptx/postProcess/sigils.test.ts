import { describe, expect, it } from "vitest";
import {
  SG_CXN_PREFIX,
  SG_GRP_PREFIX,
  SG_ID_PREFIX,
  TOKEN_DELIM,
  buildObjectName,
  parseCxnSigil,
  parseGrpSigils,
  parseIdSigil,
  stripSigils,
  stripSigilsByPrefix,
} from "./sigils.ts";

describe("buildObjectName", () => {
  it("joins non-empty tokens with the channel delimiter", () => {
    expect(buildObjectName(["sg-id:A", undefined, "node#7"])).toBe(
      `${SG_ID_PREFIX}A${TOKEN_DELIM}node#7`,
    );
  });

  it("returns empty string when every token is empty", () => {
    expect(buildObjectName([undefined, "", undefined])).toBe("");
  });
});

describe("parseIdSigil", () => {
  it("parses a bare sg-id token", () => {
    expect(parseIdSigil(`${SG_ID_PREFIX}A`)).toEqual({ userId: "A" });
  });

  it("finds sg-id in a multi-token chain regardless of position", () => {
    expect(
      parseIdSigil(`${SG_GRP_PREFIX}G1${TOKEN_DELIM}${SG_ID_PREFIX}foo`),
    ).toEqual({ userId: "foo" });
    expect(parseIdSigil(`${SG_ID_PREFIX}foo${TOKEN_DELIM}node#7`)).toEqual({
      userId: "foo",
    });
  });

  it("returns null when no sg-id token is present", () => {
    expect(parseIdSigil(undefined)).toBeNull();
    expect(parseIdSigil("")).toBeNull();
    expect(parseIdSigil("node#5")).toBeNull();
    expect(
      parseIdSigil(`${SG_CXN_PREFIX}A#right>B#left:elbow:bentConnector3`),
    ).toBeNull();
  });
});

describe("parseGrpSigils", () => {
  it("returns every sg-grp payload in outer-to-inner order", () => {
    expect(
      parseGrpSigils(
        `${SG_ID_PREFIX}A${TOKEN_DELIM}${SG_GRP_PREFIX}outer${TOKEN_DELIM}${SG_GRP_PREFIX}inner${TOKEN_DELIM}node#3`,
      ),
    ).toEqual(["outer", "inner"]);
  });

  it("returns empty array when no group token is present", () => {
    expect(parseGrpSigils(`${SG_ID_PREFIX}A`)).toEqual([]);
    expect(parseGrpSigils(undefined)).toEqual([]);
  });
});

describe("parseCxnSigil", () => {
  it("parses a full connector token in a multi-token chain", () => {
    const sig = parseCxnSigil(
      `${SG_CXN_PREFIX}A#right>B#left:elbow:bentConnector3${TOKEN_DELIM}${SG_GRP_PREFIX}diagram${TOKEN_DELIM}node#9`,
    );
    expect(sig).toEqual({
      from: "A",
      fromSide: "right",
      to: "B",
      toSide: "left",
      kind: "elbow",
      preset: "bentConnector3",
    });
  });

  it("accepts straightConnector1 as a preset value", () => {
    const sig = parseCxnSigil(
      `${SG_CXN_PREFIX}A#top>B#bottom:straight:straightConnector1`,
    );
    expect(sig?.preset).toBe("straightConnector1");
  });

  it("rejects unknown sides / kinds", () => {
    expect(
      parseCxnSigil(`${SG_CXN_PREFIX}A#diagonal>B#left:elbow:bentConnector3`),
    ).toBeNull();
    expect(
      parseCxnSigil(`${SG_CXN_PREFIX}A#right>B#left:zigzag:bentConnector3`),
    ).toBeNull();
  });

  it("returns null on structural mismatches", () => {
    expect(parseCxnSigil(`${SG_CXN_PREFIX}AB`)).toBeNull();
    expect(parseCxnSigil(`${SG_CXN_PREFIX}A#right`)).toBeNull();
    expect(parseCxnSigil(undefined)).toBeNull();
  });
});

describe("stripSigils", () => {
  it("removes every sg-* token, preserving foreign tokens like node#N", () => {
    expect(
      stripSigils(
        `${SG_ID_PREFIX}A${TOKEN_DELIM}${SG_GRP_PREFIX}G${TOKEN_DELIM}node#7`,
      ),
    ).toBe("node#7");
  });

  it("returns an empty string when no foreign tokens survive", () => {
    expect(stripSigils(`${SG_ID_PREFIX}A`)).toBe("");
    expect(
      stripSigils(
        `${SG_CXN_PREFIX}A#right>B#left:elbow:bentConnector3${TOKEN_DELIM}${SG_GRP_PREFIX}G`,
      ),
    ).toBe("");
  });

  it("leaves unrecognised names unchanged", () => {
    expect(stripSigils("Picture 1")).toBe("Picture 1");
    expect(stripSigils("")).toBe("");
    expect(stripSigils(undefined)).toBe("");
  });
});

describe("stripSigilsByPrefix", () => {
  it("removes only the requested prefix family", () => {
    const name = `${SG_ID_PREFIX}A${TOKEN_DELIM}${SG_GRP_PREFIX}G${TOKEN_DELIM}node#7`;
    expect(stripSigilsByPrefix(name, [SG_ID_PREFIX])).toBe(
      `${SG_GRP_PREFIX}G${TOKEN_DELIM}node#7`,
    );
    expect(stripSigilsByPrefix(name, [SG_GRP_PREFIX])).toBe(
      `${SG_ID_PREFIX}A${TOKEN_DELIM}node#7`,
    );
  });

  it("removes nothing when no token matches the prefix", () => {
    expect(stripSigilsByPrefix("node#7", [SG_ID_PREFIX])).toBe("node#7");
  });
});
