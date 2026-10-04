// A page reads which place it was opened for from its address:
// `?street=7` or `?junction=3`.
export const placeParam = (name) => Number(new URLSearchParams(location.search).get(name)) || 0;

// The OpenStreetMap reader is a module of its own, loaded the first time the roads of a place
// have to be read. The editor module asks for it through this.
let reader;
globalThis.cityloomImportOsm = async (bytes, bounds) => {
  reader ??= import("./pkg/osm_import.js").then(async (m) => {
    await m.default();
    return m;
  });
  return (await reader).osm_to_network(bytes, bounds);
};
