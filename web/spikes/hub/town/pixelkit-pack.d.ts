// The pixel-art tool's pack, built at build time by the spike's Vite config.
declare module "virtual:pixelkit-pack" {
  import type { ArtPack } from "../../../../tools/pixelkit/art/mandate/pack";
  const pack: ArtPack;
  /** Time the build took to make the pack, ms, and its size as JSON, bytes. */
  export const buildMs: number;
  export const bytes: number;
  export default pack;
}
