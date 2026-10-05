// The map on the city map page: MapLibre over the PMTiles basemap. What the page decides is decided in the
// WebAssembly module. This turns what MapLibre does into the JSON events the module listens for, and what the
// module asks into calls on MapLibre.

import { AttributionControl, Map as MapLibreMap, ScaleControl, addProtocol } from "./vendor/maplibre-gl.mjs";

const TILES = "data/basemap/montreal.pmtiles";
// MapLibre needs absolute addresses for what a style names.
const here = (path) => new URL(path, document.baseURI).href;

const themeNow = () => {
  const set = document.documentElement.dataset.theme;
  if (set === "light" || set === "dark") return set;
  return matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
};

async function styleFor(theme) {
  const style = await (await fetch(`basemap-${theme}.json`, { cache: "no-cache" })).json();
  style.glyphs = here("glyphs/") + "{fontstack}/{range}.pbf";
  style.sources.basemap.url = "pmtiles://" + here(TILES);
  return style;
}

// Always an adapter: a map that cannot start (no style, no PMTiles reader, no WebGL) says `failed`, and the
// module shows why, rather than the page never mounting.
export async function createBasemap(container) {
  // What is said before the module listens waits for it.
  let listener = null;
  const queue = [];
  const emit = (event) => {
    const json = JSON.stringify(event);
    if (listener) listener(json);
    else queue.push(json);
  };
  const listen = (on) => {
    listener = on;
    queue.splice(0).forEach(on);
  };

  let theme = themeNow();
  let map, header;
  try {
    const protocol = new pmtiles.Protocol();
    addProtocol("pmtiles", protocol.tile);
    const archive = new pmtiles.PMTiles(here(TILES));
    protocol.add(archive);
    header = archive.getHeader();
    header.catch(() => {}); // answered below, once the map has started
    map = new MapLibreMap({
      container,
      style: await styleFor(theme),
      center: [0, 0],
      zoom: 1,
      maxZoom: 19,
      attributionControl: false,
      dragRotate: false,
      pitchWithRotate: false,
      touchPitch: false,
      keyboard: false, // the shortcuts are the page's
    });
  } catch (error) {
    console.warn("The basemap could not start:", error);
    emit({ kind: "failed" });
    const nothing = () => {};
    return {
      setPlaces: nothing,
      fit: nothing,
      zoomBy: nothing,
      panBy: nothing,
      highlight: nothing,
      setImperial: nothing,
      listen,
    };
  }
  header.then(
    (h) => emit({ kind: "ready", bounds: [h.minLon, h.minLat, h.maxLon, h.maxLat] }),
    () => emit({ kind: "failed" }),
  );
  // The page's `#basemap` is the map's one stop for the keyboard, and holds its name: MapLibre's canvas
  // inside it would be a second.
  map.getCanvas().setAttribute("tabindex", "-1");
  map.touchZoomRotate.disableRotation();
  map.addControl(new AttributionControl({ compact: true }));
  let scale = new ScaleControl({ unit: "metric" });
  map.addControl(scale, "bottom-left");
  window.cityloomMap = map;

  // The places: kept, so that a new style (a new theme) can have them put back.
  let places = null;
  let hot = null;
  let styled = false;
  const showHot = () => {
    console.log("DIAG showHot", JSON.stringify({ styled, source: !!map.getSource("places"), hot }));
    if (styled && map.getSource("places") && hot) map.setFeatureState({ source: "places", id: hot }, { hot: true });
  };
  const showPlaces = () => {
    if (!styled || !places) return;
    const data = JSON.parse(places.data);
    const source = map.getSource("places");
    if (source) {
      source.setData(data);
    } else {
      map.addSource("places", { type: "geojson", data, promoteId: "hot" });
      for (const layer of JSON.parse(places.layers)) map.addLayer(layer);
    }
    showHot();
  };
  map.on("style.load", () => {
    styled = true;
    showPlaces();
  });

  // `theme` is the one asked for last, `shown` the one whose style the map has.
  let shown = theme;
  const retheme = async () => {
    const next = themeNow();
    if (next === theme) return;
    theme = next;
    let style;
    try {
      style = await styleFor(next);
    } catch (error) {
      // The map keeps the style it has; a later flip tries again.
      console.warn("The basemap's style could not be had:", error);
      if (theme === next) theme = shown;
      return;
    }
    if (theme !== next) return; // a newer flip is in flight and applies its own style
    shown = next;
    styled = false;
    map.setStyle(style, { diff: false });
  };
  new MutationObserver(retheme).observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme"],
  });
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", retheme);

  // What is under the pointer: a junction before a street.
  const hotAt = (point) => {
    const layers = ["places-junction", "places-street"].filter((id) => map.getLayer(id));
    if (!layers.length) return null;
    const box = [
      [point.x - 6, point.y - 6],
      [point.x + 6, point.y + 6],
    ];
    const found = map.queryRenderedFeatures(box, { layers });
    const feature = found.find((f) => f.layer.id === "places-junction") ?? found[0];
    return feature ? feature.properties.hot : null;
  };
  let over = null;
  map.on("mousemove", (e) => {
    const now = hotAt(e.point);
    if (now === over) return;
    over = now;
    map.getCanvas().style.cursor = now ? "pointer" : "";
    emit({ kind: "hover", hot: now });
  });
  map.getCanvas().addEventListener("mouseleave", () => {
    over = null;
    emit({ kind: "hover", hot: null });
  });
  map.on("click", (e) => {
    const now = hotAt(e.point);
    if (now) emit({ kind: "pick", hot: now });
  });
  // A move that came from the person's own pointer or wheel, not from the page.
  map.on("movestart", (e) => {
    if (e.originalEvent) emit({ kind: "moved" });
  });

  return {
    setPlaces(layers, data) {
      places = { layers, data };
      showPlaces();
    },
    fit(west, south, east, north, top, right, bottom, left) {
      map.fitBounds(
        [
          [west, south],
          [east, north],
        ],
        { padding: { top, right, bottom, left }, duration: 300 },
      );
    },
    zoomBy(factor) {
      map.easeTo({ zoom: map.getZoom() + Math.log2(factor), duration: 200 });
    },
    // A positive dx moves the view east, a positive dy south.
    panBy(dx, dy) {
      map.panBy([dx, dy], { duration: 150 });
    },
    highlight(next) {
      if (styled && map.getSource("places") && hot) map.setFeatureState({ source: "places", id: hot }, { hot: false });
      hot = next;
      showHot();
    },
    setImperial(imperial) {
      map.removeControl(scale);
      scale = new ScaleControl({ unit: imperial ? "imperial" : "metric" });
      map.addControl(scale, "bottom-left");
    },
    listen,
  };
}
