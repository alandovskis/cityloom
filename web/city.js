// A page reads which place it was opened for from its address:
// `?street=7` or `?junction=3`.
export const placeParam = (name) => Number(new URLSearchParams(location.search).get(name)) || 0;
