// JSON for packs: byte arrays travel as base64 (`{"$u8": "..."}`), so a built
// pack is one text file the game can import or fetch.

const B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

export function toBase64(u: Uint8Array): string {
  let s = "";
  for (let i = 0; i < u.length; i += 3) {
    const a = u[i] as number;
    const b = i + 1 < u.length ? (u[i + 1] as number) : 0;
    const c = i + 2 < u.length ? (u[i + 2] as number) : 0;
    const n = (a << 16) | (b << 8) | c;
    s += B64[(n >> 18) & 63] as string;
    s += B64[(n >> 12) & 63] as string;
    s += i + 1 < u.length ? (B64[(n >> 6) & 63] as string) : "=";
    s += i + 2 < u.length ? (B64[n & 63] as string) : "=";
  }
  return s;
}

export function fromBase64(s: string): Uint8Array {
  const clean = s.replace(/=+$/, "");
  const out = new Uint8Array(Math.floor((clean.length * 3) / 4));
  let o = 0;
  for (let i = 0; i < clean.length; i += 4) {
    const n = [0, 1, 2, 3].reduce((acc, k) => (acc << 6) | (i + k < clean.length ? B64.indexOf(clean[i + k] as string) : 0), 0);
    if (o < out.length) out[o++] = (n >> 16) & 255;
    if (o < out.length) out[o++] = (n >> 8) & 255;
    if (o < out.length) out[o++] = n & 255;
  }
  return out;
}

export function packToJson(value: unknown): string {
  return JSON.stringify(value, (_k, v: unknown) => (v instanceof Uint8Array ? { $u8: toBase64(v) } : v));
}

export function packFromJson<T>(text: string): T {
  return JSON.parse(text, (_k, v: unknown) => (v && typeof v === "object" && "$u8" in v ? fromBase64((v as { $u8: string }).$u8) : v)) as T;
}
