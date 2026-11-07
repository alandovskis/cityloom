// The report writer is a module of its own, loaded the first time a report is asked for. The editor module asks
// for it through `cityloomReportPdf`. It needs the two fonts the PDF embeds; they are fetched once and kept.
// A failure is not kept: a later call tries again. The usual cause is a deploy that came after the page was opened,
// which retired this build's addresses, so the page says it must be reloaded. The browser remembers a module that
// failed to load, for the address it was asked by, so a retry asks by another one.
let writer;
let attempts = 0;
const load = () =>
  (writer ??= import(`./pkg/cityloom_report_pdf.js${attempts++ ? `?retry=${attempts}` : ""}`)
    .then(async (m) => {
      await m.default();
      return m;
    })
    .catch((error) => {
      writer = undefined;
      globalThis.cityloomBootFailed?.();
      throw error;
    }));

let fonts;
const loadFonts = () =>
  (fonts ??= Promise.all(
    ["/fonts/NotoSans-Regular.ttf", "/fonts/NotoSans-Bold.ttf"].map(async (url) => {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`the font ${url} could not be fetched (${response.status})`);
      return new Uint8Array(await response.arrayBuffer());
    }),
  ).catch((error) => {
    fonts = undefined;
    throw error;
  }));

globalThis.cityloomReportPdf = async (json) => {
  const [module, [regular, bold]] = await Promise.all([load(), loadFonts()]);
  return module.report_pdf(json, regular, bold);
};
