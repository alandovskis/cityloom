// A page reads which place it was opened for from its address:
// `?street=7` or `?junction=3`.
export const placeParam = (name) => Number(new URLSearchParams(location.search).get(name)) || 0;

// The OpenStreetMap reader is a module of its own, loaded the first time the roads of a place
// have to be read. The editor module asks for it through this.
// A failure is not kept: a later call tries again. The usual cause is a deploy that came after the page was opened,
// which retired this build's addresses, so the page says it must be reloaded. The browser remembers a module that
// failed to load, for the address it was asked by, so a retry asks by another one.
let reader;
let attempts = 0;
const load = () =>
  (reader ??= import(`./pkg/osm_import.js${attempts++ ? `?retry=${attempts}` : ""}`)
    .then(async (m) => {
      await m.default();
      return m;
    })
    .catch((error) => {
      reader = undefined;
      globalThis.cityloomBootFailed?.();
      throw error;
    }));

// The module asks for the reader to be loaded as soon as it knows the roads have to be read, without waiting.
globalThis.cityloomPrepareOsm = () => load().catch(() => {});
globalThis.cityloomImportOsm = async (bytes, bounds) => (await load()).osm_to_network(bytes, bounds);
