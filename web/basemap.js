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
    queue.splice(0).forEach((json) => on(json));
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
      setZoneTicks: nothing,
      centre: () => new Float64Array([0, 0]),
      fit: nothing,
      setInsets: nothing,
      setEditable: nothing,
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
  // Which places can be edited, as the module last said; `null` until it has said, when every place is editable.
  // Kept, and put back whenever the places are drawn again: the module says it only when the set changes, and a
  // new style forgets feature-state.
  let editable = null;
  // The sources whose features carry a place's `hot` id (the places, and what is drawn beside them): each takes the
  // place's feature state.
  const idSources = ["places", "curbside", "stops"];
  const idsOf = { places: [], curbside: [], stops: [] };
  const showEditable = () => {
    if (!styled || !editable) return;
    for (const name of idSources) {
      if (!map.getSource(name)) continue;
      for (const id of idsOf[name]) map.setFeatureState({ source: name, id }, { editable: editable.has(id) });
    }
  };
  const showHot = () => {
    if (!styled || !hot) return;
    for (const name of idSources)
      if (map.getSource(name)) map.setFeatureState({ source: name, id: hot }, { hot: true });
  };
  const showPlaces = () => {
    if (!styled || !places) return;
    const data = JSON.parse(places.data);
    idsOf.places = data.features.map((f) => f.properties.hot);
    const source = map.getSource("places");
    if (source) {
      source.setData(data);
    } else {
      map.addSource("places", { type: "geojson", data, promoteId: "hot" });
      for (const layer of JSON.parse(places.layers)) map.addLayer(layer);
    }
    // what was shown before the places goes under their junction marks
    for (const [kept, name] of [[curbside, "curbside"], [stops, "stops"]])
      if (kept && map.getSource(name))
        for (const layer of JSON.parse(kept.layers)) map.moveLayer(layer.id, "places-junction");
    showHot();
    showEditable();
  };
  // The restrictions and the outline being drawn: kept like the places, so a new style can have them put back.
  // The restrictions go under the places, the outline over everything.
  let restrictions = null;
  let draft = null;
  let zoneTicks = null;
  let curbside = null;
  let stops = null;
  const show = (kept, name, before, promoteId) => {
    if (!styled || !kept) return;
    const data = JSON.parse(kept.data);
    const source = map.getSource(name);
    if (source) {
      source.setData(data);
    } else {
      map.addSource(name, promoteId ? { type: "geojson", data, promoteId } : { type: "geojson", data });
      for (const layer of JSON.parse(kept.layers)) map.addLayer(layer, before);
    }
  };
  // The curbside uses and the stops are the street's: their features say its `hot` id, and take its feature state.
  const showBesides = (kept, name) => {
    // over the streets, under the junction marks (the disc covers a use's first metres as it covers the street's
    // line, so what is pressed is what is seen), the zone boundaries and the outline being drawn
    const above = ["places-junction", "zone-ticks-casing", "draft-fill"].find((id) => map.getLayer(id));
    show(kept, name, above, "hot");
    if (!styled || !kept) return;
    idsOf[name] = [...new Set(JSON.parse(kept.data).features.map((f) => f.properties.hot))];
    showHot();
    showEditable();
  };
  const showCurbside = () => showBesides(curbside, "curbside");
  const showStops = () => showBesides(stops, "stops");
  const showRestrictions = () =>
    show(restrictions, "restrictions", map.getLayer("places-casing") ? "places-casing" : undefined);
  const showZoneTicks = () => show(zoneTicks, "zone-ticks", map.getLayer("draft-fill") ? "draft-fill" : undefined);
  const showDraft = () => show(draft, "draft", undefined);
  map.on("style.load", () => {
    styled = true;
    showRestrictions();
    showPlaces();
    showCurbside();
    showStops();
    showZoneTicks();
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
    // A junction before a street, a street before what lies beside it.
    const order = ["places-junction", "places-street", "stops-dot", "stops-platform", "curbside-fill"];
    const layers = order.filter((id) => map.getLayer(id));
    if (!layers.length) return null;
    const found = map.queryRenderedFeatures(near(point), { layers });
    for (const id of order) {
      const feature = found.find((f) => f.layer.id === id);
      if (feature) return feature.properties.hot;
    }
    return null;
  };
  // The part of the map the panels leave open, as the module last said what floats over it: top, right, bottom, left.
  let insets = [0, 0, 0, 0];
  const emitView = () => {
    const box = map.getContainer();
    const [w, h] = [box.clientWidth, box.clientHeight];
    if (!w || !h) return; // a map with no size has no view to hold anything
    let [top, right, bottom, left] = insets;
    if (left + right >= w || top + bottom >= h) [top, right, bottom, left] = [0, 0, 0, 0]; // the panels cover it all: the whole map is open
    const nw = map.unproject([left, top]);
    const se = map.unproject([w - right, h - bottom]);
    emit({ kind: "view", bounds: [nw.lng, se.lat, se.lng, nw.lat] });
  };
  map.on("moveend", emitView);
  map.on("resize", emitView);
  let over = null;
  const showCursor = () => {
    map.getCanvas().style.cursor = over && (!editable || editable.has(over)) ? "pointer" : "";
  };
  map.on("mousemove", (e) => {
    const now = hotAt(e.point);
    if (now === over) return;
    over = now;
    showCursor();
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
  // The corner of the outline being drawn near the point: the first corner before a later one, so that a small
  // outline can still be closed on it.
  const draftCornerAt = (point) => {
    if (!map.getLayer("draft-vertex")) return null;
    const found = map.queryRenderedFeatures(near(point), { layers: ["draft-vertex"] });
    const feature = found.find((f) => f.properties.i === 0) ?? found[0];
    return feature ? feature.properties.i : null;
  };
  // Whether the last press of the left button on the map took a corner: the click that follows it is that drag's,
  // not a press. Every left press on the map sets it again, so it never holds back a later click.
  let pressedCorner = false;
  // A press says where it was and what is there; the module decides what it means.
  map.on("click", (e) => {
    if (pressedCorner) {
      pressedCorner = false;
      return;
    }
    emit({
      kind: "press",
      at: [e.lngLat.lng, e.lngLat.lat],
      hot: hotAt(e.point),
      area: firstIn(e.point, "areas-fill", "area"),
      vertex: draftCornerAt(e.point),
    });
  });
  // A corner of the selected restriction dragged: the map does not pan under it, and the module is told where it
  // is at each move and when it is let go. The window hears the drag, so a corner let go off the map still ends
  // it, as does a release the page never heard (the button is up at the next move) or the window losing focus;
  // those end it where it was last. A corner pressed and let go without a move stays where it is.
  let dragging = null;
  let last = null;
  let moved = false;
  const lngLatOf = (e) => {
    const box = map.getCanvas().getBoundingClientRect();
    const at = map.unproject([e.clientX - box.left, e.clientY - box.top]);
    return [at.lng, at.lat];
  };
  const dragEnd = (at) => {
    window.removeEventListener("mousemove", dragMove);
    window.removeEventListener("mouseup", dragUp);
    window.removeEventListener("blur", dragLost);
    const corner = dragging;
    dragging = null;
    emit({ kind: "vertex", ...corner, at, done: true });
  };
  const dragMove = (e) => {
    if ((e.buttons & 1) === 0) return dragEnd(last);
    last = lngLatOf(e);
    moved = true;
    emit({ kind: "vertex", ...dragging, at: last, done: false });
  };
  const dragUp = (e) => {
    if (e.button === 0) dragEnd(moved ? lngLatOf(e) : last);
  };
  const dragLost = () => dragEnd(last);
  map.on("mousedown", (e) => {
    if (dragging || e.originalEvent.button !== 0) return;
    pressedCorner = false;
    if (!map.getLayer("areas-vertex")) return;
    const feature = map.queryRenderedFeatures(near(e.point), { layers: ["areas-vertex"] })[0];
    if (!feature) return;
    e.preventDefault();
    pressedCorner = true;
    dragging = { area: feature.properties.area, index: feature.properties.i };
    last = feature.geometry.coordinates;
    moved = false;
    window.addEventListener("mousemove", dragMove);
    window.addEventListener("mouseup", dragUp);
    window.addEventListener("blur", dragLost);
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
    setZoneTicks(layers, data) {
      zoneTicks = { layers, data };
      showZoneTicks();
    },
    setCurbside(layers, data) {
      curbside = { layers, data };
      showCurbside();
    },
    setStops(layers, data) {
      stops = { layers, data };
      showStops();
    },
    setDraft(layers, data) {
      draft = { layers, data };
      showDraft();
    },
    centre() {
      const c = map.getCenter();
      return new Float64Array([c.lng, c.lat]);
    },
    // `bounds` is west, south, east, north and `insets` top, right, bottom, left, as numbers.
    fit([west, south, east, north], [top, right, bottom, left]) {
      map.fitBounds(
        [
          [west, south],
          [east, north],
        ],
        { padding: { top, right, bottom, left }, duration: ms(300) },
      );
    },
    setInsets(next) {
      insets = Array.from(next);
      emitView();
    },
    setEditable(hot) {
      editable = new Set(hot);
      showEditable();
      showCursor();
    },
    zoomBy(factor) {
      map.easeTo({ zoom: map.getZoom() + Math.log2(factor), duration: ms(200) });
    },
    // A positive dx moves the view east, a positive dy south.
    panBy(dx, dy) {
      map.panBy([dx, dy], { duration: ms(150) });
    },
    highlight(next) {
      if (styled && hot)
        for (const name of idSources)
          if (map.getSource(name)) map.setFeatureState({ source: name, id: hot }, { hot: false });
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
