// @ts-check
/** @typedef {import('./types').TranscriptionRoute} TranscriptionRoute */

/** @param {TranscriptionRoute[]} routes @returns {TranscriptionRoute[]} */
export function addRoute(routes) {
  return [...routes, { providerId: '', model: '' }];
}

/** @param {TranscriptionRoute[]} routes @param {number} index @returns {TranscriptionRoute[]} */
export function removeRoute(routes, index) {
  if (routes.length <= 1 || index < 0 || index >= routes.length) return [...routes];
  return routes.filter((_, current) => current !== index);
}

/** @param {TranscriptionRoute[]} routes @param {number} from @param {number} to @returns {TranscriptionRoute[]} */
export function moveRoute(routes, from, to) {
  if (from < 0 || from >= routes.length || to < 0 || to >= routes.length || from === to) {
    return [...routes];
  }
  const next = [...routes];
  const [route] = next.splice(from, 1);
  next.splice(to, 0, route);
  return next;
}

/** @param {TranscriptionRoute[]} routes @param {number} index @param {Partial<TranscriptionRoute>} patch @returns {TranscriptionRoute[]} */
export function updateRoute(routes, index, patch) {
  if (index < 0 || index >= routes.length) return [...routes];
  return routes.map((route, current) => current === index ? { ...route, ...patch } : route);
}
