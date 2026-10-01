// The city the street and junction editors share. It lives in the browser's
// storage as the text the Rust core writes; a page opens it, takes a street or
// a junction out to edit, and writes what it made back.

import { City } from "./pkg/cityloom_editor.js";
import { recall, remember } from "./shell.js";

const KEY = "cityloom-city";

export const openCity = () => new City(recall(KEY) || "");

// Applies `change` to the city as it is in storage now and stores the result,
// so a change made in another tab is not written over. Says whether the
// change was taken and stored.
export function writeCity(change) {
  const city = openCity();
  return change(city) !== false && remember(KEY, city.save());
}

export function resetCity() {
  return writeCity((city) => city.reset());
}

// The index of the region the person chose, which the pages share.
export function regionIndex(regions) {
  const i = regions.findIndex((r) => r.id === recall("cityloom-region"));
  return i < 0 ? 0 : i;
}

// A page reads which place it was opened for from its address:
// `?street=7` or `?junction=3`.
export const placeParam = (name) => Number(new URLSearchParams(location.search).get(name)) || 0;

// Runs `keep` a moment after the last call, and at once if the page is left
// before then. `onFail` is told once if the change could not be stored.
export function keeper(keep, onFail) {
  let timer = 0;
  let told = false;
  const run = () => {
    clearTimeout(timer);
    timer = 0;
    if (!keep() && !told) {
      told = true;
      onFail();
    }
  };
  addEventListener("pagehide", () => timer && run());
  return () => {
    clearTimeout(timer);
    timer = setTimeout(run, 250);
  };
}

export const NOT_KEPT = "This browser is not keeping your changes, so other pages will not see them.";
