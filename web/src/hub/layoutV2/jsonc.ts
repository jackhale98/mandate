// A small JSONC reader for the layout registries (DEC-053 content is JSONC:
// `//` and `/* */` comments and trailing commas). Stage B moves the
// registries into mandate-content, where Rust parses them; this keeps the
// same files readable on the client until then.

/** Parse JSONC text: strips comments and trailing commas outside strings, then `JSON.parse`. */
export function parseJsonc(text: string): unknown {
  let out = "";
  let i = 0;
  const n = text.length;
  while (i < n) {
    const c = text[i] as string;
    if (c === '"') {
      // Copy a string literal verbatim (escapes included).
      let j = i + 1;
      while (j < n && text[j] !== '"') j += text[j] === "\\" ? 2 : 1;
      out += text.slice(i, j + 1);
      i = j + 1;
    } else if (c === "/" && text[i + 1] === "/") {
      while (i < n && text[i] !== "\n") i++;
    } else if (c === "/" && text[i + 1] === "*") {
      const end = text.indexOf("*/", i + 2);
      i = end < 0 ? n : end + 2;
    } else {
      out += c;
      i++;
    }
  }
  // Trailing commas: a comma followed (after whitespace) by } or ].
  let clean = "";
  for (let k = 0; k < out.length; k++) {
    const c = out[k] as string;
    if (c === '"') {
      let j = k + 1;
      while (j < out.length && out[j] !== '"') j += out[j] === "\\" ? 2 : 1;
      clean += out.slice(k, j + 1);
      k = j;
      continue;
    }
    if (c === ",") {
      let j = k + 1;
      while (j < out.length && /\s/.test(out[j] as string)) j++;
      if (out[j] === "}" || out[j] === "]") continue;
    }
    clean += c;
  }
  return JSON.parse(clean);
}
