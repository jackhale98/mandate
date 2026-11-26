// Sharing the career card with no server (M6 note 23): "Copy" puts the text
// block on the clipboard; "Share" opens the device's share sheet with the
// card's image and its text (Web Share Level 2), and where that is missing or
// refused saves the image and copies the text instead.

export const CARD_FILE = "mandate-career-card.png";

/** Put text on the clipboard; false where the device will not. */
export async function copyText(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Refused (no permission, not a secure context): the old way below.
  }
  try {
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    const ok = document.execCommand?.("copy") ?? false;
    area.remove();
    return ok;
  } catch {
    return false;
  }
}

/** Save a file: Blob + object URL + <a download>. */
export function download(name: string, file: Blob) {
  const url = URL.createObjectURL(file);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.rel = "noopener";
  a.style.display = "none";
  document.body.appendChild(a);
  a.click();
  a.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 60_000);
}

/** How a share went: the sheet was used, cancelled, or the fallback saved the image (and copied the text, or not), or only copied the text. */
export type Shared = "shared" | "cancelled" | "saved" | "saved-only" | "copied" | "failed";

/** Share the card: the image (when it could be drawn) and the text. */
export async function shareCard(text: string, title: string, image: () => Promise<Blob>): Promise<Shared> {
  let file: File | null;
  try {
    file = new File([await image()], CARD_FILE, { type: "image/png" });
  } catch {
    // No canvas on this device: the text alone.
    file = null;
  }
  const canShare = typeof navigator.share === "function";
  const withFile = file !== null && typeof navigator.canShare === "function" && navigator.canShare({ files: [file] });
  if (canShare && (withFile || file === null)) {
    try {
      await navigator.share(withFile && file ? { files: [file], title, text } : { title, text });
      return "shared";
    } catch (e) {
      if (e instanceof DOMException && e.name === "AbortError") return "cancelled";
      // Not allowed (the tap's permission expired, or the files refused): the fallback.
    }
  }
  const copied = await copyText(text);
  if (file) {
    download(CARD_FILE, file);
    return copied ? "saved" : "saved-only";
  }
  return copied ? "copied" : "failed";
}
