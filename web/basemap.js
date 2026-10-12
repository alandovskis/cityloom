// The map on the city map page: MapLibre over the PMTiles basemap. What the page decides is decided in the
// WebAssembly module. This turns what MapLibre does into the JSON events the module listens for, and what the
// module asks into calls on MapLibre.

import { AttributionControl, Map as MapLibreMap, ScaleControl, addProtocol } from "./vendor/maplibre-gl.mjs";

const TILES = "data/basemap/montreal.pmtiles";
// MapLibre needs absolute addresses for what a style names.
const here = (path) => new URL(path, document.baseURI).href;

// Someone who asks for less motion gets the camera there at once.
const still = matchMedia("(prefers-reduced-motion: reduce)");
const ms = (duration) => (still.matches ? 0 : duration);

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
      setRestrictions: nothing,
      setDraft: nothing,
      centre: () => new Float64Array([0, 0]),
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
  // The restrictions and the outline being drawn: kept like the places, so a new style can have them put back.
  // The restrictions go under the places, the outline over everything.
  let restrictions = null;
  let draft = null;
  const show = (kept, name, before) => {
    if (!styled || !kept) return;
    const data = JSON.parse(kept.data);
    const source = map.getSource(name);
    if (source) {
      source.setData(data);
    } else {
      map.addSource(name, { type: "geojson", data });
      for (const layer of JSON.parse(kept.layers)) map.addLayer(layer, before);
    }
  };
  const showRestrictions = () =>
    show(restrictions, "restrictions", map.getLayer("places-casing") ? "places-casing" : undefined);
  const showDraft = () => show(draft, "draft", undefined);
  map.on("style.load", () => {
    styled = true;
    showRestrictions();
    showPlaces();
    showDraft();
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
  const near = (point) => [
    [point.x - 6, point.y - 6],
    [point.x + 6, point.y + 6],
  ];
  const hotAt = (point) => {
    const layers = ["places-junction", "places-street"].filter((id) => map.getLayer(id));
    if (!layers.length) return null;
    const found = map.queryRenderedFeatures(near(point), { layers });
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
  // Leaving the map puts out only what the map itself lit: a row of the list the pointer is on now has lit its
  // own place, and the browser says this after the row's pointerenter.
  map.getCanvas().addEventListener("mouseleave", () => {
    if (over === null) return;
    over = null;
    emit({ kind: "hover", hot: null });
  });
  // A property of the first feature of `layer` near the point, if the layer is there.
  const firstIn = (point, layer, property) => {
    if (!map.getLayer(layer)) return null;
    const feature = map.queryRenderedFeatures(near(point), { layers: [layer] })[0];
    return feature ? feature.properties[property] : null;
  };
  // A press says where it was and what is there; the module decides what it means.
  map.on("click", (e) => {
    emit({
      kind: "press",
      at: [e.lngLat.lng, e.lngLat.lat],
      hot: hotAt(e.point),
      area: firstIn(e.point, "areas-fill", "area"),
      vertex: firstIn(e.point, "draft-vertex", "i"),
    });
  });
  // A corner of the selected restriction dragged: the map does not pan under it, and the module is told where it
  // is at each move and when it is let go. The window hears the drag, so a corner let go off the map still ends it.
  let dragging = null;
  const lngLatOf = (e) => {
    const box = map.getCanvas().getBoundingClientRect();
    const at = map.unproject([e.clientX - box.left, e.clientY - box.top]);
    return [at.lng, at.lat];
  };
  const dragTo = (e, done) => emit({ kind: "vertex", ...dragging, at: lngLatOf(e), done });
  const dragMove = (e) => dragTo(e, false);
  const dragEnd = (e) => {
    window.removeEventListener("mousemove", dragMove);
    window.removeEventListener("mouseup", dragEnd);
    dragTo(e, true);
    dragging = null;
  };
  map.on("mousedown", (e) => {
    if (dragging || e.originalEvent.button !== 0 || !map.getLayer("areas-vertex")) return;
    const feature = map.queryRenderedFeatures(near(e.point), { layers: ["areas-vertex"] })[0];
    if (!feature) return;
    e.preventDefault();
    dragging = { area: feature.properties.area, index: feature.properties.i };
    window.addEventListener("mousemove", dragMove);
    window.addEventListener("mouseup", dragEnd);
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
    setRestrictions(layers, data) {
      restrictions = { layers, data };
      showRestrictions();
    },
    setDraft(layers, data) {
      draft = { layers, data };
      showDraft();
    },
    centre() {
      const c = map.getCenter();
      return new Float64Array([c.lng, c.lat]);
    },
    fit(west, south, east, north, top, right, bottom, left) {
      map.fitBounds(
        [
          [west, south],
          [east, north],
        ],
        { padding: { top, right, bottom, left }, duration: ms(300) },
      );
    },
    zoomBy(factor) {
      map.easeTo({ zoom: map.getZoom() + Math.log2(factor), duration: ms(200) });
    },
    // A positive dx moves the view east, a positive dy south.
    panBy(dx, dy) {
      map.panBy([dx, dy], { duration: ms(150) });
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
