// Lets Node run the tool's TypeScript directly (type stripping, Node ≥ 22.18):
// relative imports are written without extensions, as bundlers and the web
// app's TypeScript settings expect, so this resolve hook adds `.ts`. No
// dependencies, no build step.
import { registerHooks } from "node:module";

registerHooks({
  resolve(specifier, context, next) {
    if ((specifier.startsWith("./") || specifier.startsWith("../")) && !/\.[cm]?[jt]s$|\.json$/.test(specifier)) {
      try {
        return next(`${specifier}.ts`, context);
      } catch {
        return next(`${specifier}/index.ts`, context);
      }
    }
    return next(specifier, context);
  },
});
