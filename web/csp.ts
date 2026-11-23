// Content Security Policy for the built pages (SECURITY_REVIEW.md, A4).
// GitHub Pages cannot send headers, so the policy is a <meta> tag injected at
// build time (never in `vite dev`, whose hot reload needs inline scripts).
// A meta policy cannot set frame-ancestors, report-uri or sandbox; it covers
// everything else below.
//
// Nothing here names a remote origin: the game loads no fonts, scripts, images
// or data from anywhere but its own files, and sends nothing anywhere.
import { createHash } from "node:crypto";
import type { HtmlTagDescriptor, Plugin } from "vite";

export type Policy = Record<string, string[]>;

/** Directives shared by every page. */
const common: Policy = {
  "default-src": ["'self'"],
  "base-uri": ["'self'"],
  "object-src": ["'none'"],
  "form-action": ["'self'"],
  "manifest-src": ["'self'"],
  "font-src": ["'self'"],
  "connect-src": ["'self'"], // the engine's .wasm file is fetched from our own origin
};

/** The game (play/): scripts and a worker from this origin, WebAssembly, canvas art. */
export const appPolicy: Policy = {
  ...common,
  "script-src": ["'self'", "'wasm-unsafe-eval'"], // 'wasm-unsafe-eval' lets WebAssembly compile; it does not allow eval()
  "worker-src": ["'self'"], // the engine worker and the town-plan worker
  "style-src": ["'self'"],
  "img-src": ["'self'", "data:"],
};

/** The landing page: static HTML, one small inline script (allowed by hash, added at build). */
export const landingPolicy: Policy = {
  ...common,
  "script-src": ["'self'"],
  "worker-src": ["'self'"],
  "style-src": ["'self'"],
  // The front page's bar chart sets one CSS variable per bar in a style attribute (--v: 8). Attributes cannot run script.
  "style-src-attr": ["'unsafe-inline'"],
  "img-src": ["'self'", "data:"],
  "form-action": ["'none'"],
};

export function policyString(policy: Policy): string {
  return Object.entries(policy)
    .map(([k, v]) => `${k} ${v.join(" ")}`)
    .join("; ");
}

/** Hashes (`'sha256-...'`) of every inline classic script in `html`. */
export function inlineScriptHashes(html: string): string[] {
  const out: string[] = [];
  for (const m of html.matchAll(/<script(?![^>]*\ssrc=)([^>]*)>([\s\S]*?)<\/script>/gi)) {
    if (/type\s*=\s*["']?(application\/(ld\+)?json|importmap)/i.test(m[1] ?? "")) continue;
    out.push(`'sha256-${createHash("sha256").update(m[2] ?? "").digest("base64")}'`);
  }
  return out;
}

/** Injects the policy as the first tag of <head> in a production build. */
export function cspMeta(policy: Policy, opts: { hashInlineScripts?: boolean } = {}): Plugin {
  return {
    name: "mandate-csp",
    apply: "build",
    transformIndexHtml: {
      order: "post",
      handler(html) {
        const p: Policy = { ...policy };
        if (opts.hashInlineScripts) p["script-src"] = [...(p["script-src"] ?? []), ...inlineScriptHashes(html)];
        const tag: HtmlTagDescriptor = {
          tag: "meta",
          attrs: { "http-equiv": "Content-Security-Policy", content: policyString(p) },
          injectTo: "head-prepend",
        };
        return { html, tags: [tag] };
      },
    },
  };
}
