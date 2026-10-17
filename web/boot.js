// Shown when a page's code cannot be loaded or started (the network failed, the browser is too old, or the page was
// made for a build the server has since replaced). This is the one script that is not under /v/<build>/: it has to be
// there when those addresses are not, and a classic script so that no module import can stand between it and the page.
// The words are in the page's markup, one block per language, because the module that holds every other message is
// the thing that did not load. This only picks the block, as the module would: the language chosen on the site, else
// the first the browser prefers that the site speaks. A page's entry script and the loader of the roads call it.
(() => {
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

  globalThis.cityloomBootFailed = () => {
    const box = document.getElementById("boot-failed");
    if (!box) return;
    const want = language();
    for (const part of box.children) part.hidden = !part.lang.toLowerCase().startsWith(want);
    box.hidden = false;
  };
})();
