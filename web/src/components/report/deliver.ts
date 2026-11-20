// Hand a report file to the tester: the share sheet on touch devices that can
// share files (Web Share Level 2), a download everywhere else (and whenever
// sharing is refused). Nothing is uploaded; the tester decides where it goes.

/** Save a file the way phones and desktops both support: Blob + object URL + <a download>. */
function download(name: string, file: Blob) {
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

export type Delivered = "shared" | "cancelled" | "downloaded";

export async function deliverFile(name: string, text: string): Promise<Delivered> {
  const file = new File([text], name, { type: "application/json" });
  const coarse = window.matchMedia?.("(pointer: coarse)").matches === true;
  if (coarse && typeof navigator.canShare === "function" && navigator.canShare({ files: [file] })) {
    try {
      await navigator.share({ files: [file], title: name });
      return "shared";
    } catch (e) {
      if (e instanceof DOMException && e.name === "AbortError") return "cancelled";
      // Not allowed (for example the tap's permission expired): a download instead.
    }
  }
  download(name, file);
  return "downloaded";
}
