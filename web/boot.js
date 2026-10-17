// Shown when a page's WebAssembly module cannot be loaded or linked (the network failed, the browser is too old,
// the page was made for another build). The words are in the page's markup, one block per language: the module
// that holds every other message is the thing that did not load. This only picks the block, as the module would:
// the language chosen on the site, else the first the browser prefers that the site speaks.
const spoken = (tag) => (/^(en|fr)(-|$)/i.exec(tag ?? "") ?? [])[1]?.toLowerCase() ?? "";

function language() {
  let chosen = "";
  try {
    chosen = spoken(localStorage.getItem("cityloom-lang"));
  } catch {
    // storage is blocked: the browser's languages decide
  }
  return chosen || (navigator.languages ?? []).map(spoken).find(Boolean) || "en";
}

export function failed() {
  const box = document.getElementById("boot-failed");
  if (!box) return;
  const want = language();
  for (const part of box.children) part.hidden = !part.lang.toLowerCase().startsWith(want);
  box.hidden = false;
}
