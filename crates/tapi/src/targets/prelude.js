// @ts-check
let GLOBAL_API_BASE = "";
/**
 * @param {ApiOptions} [options]
 * @returns {string}
 */
export const getApiBase = (options) => options?.apiBase ?? GLOBAL_API_BASE;
/**
 * @param {string} apiBase
 * @returns {string}
 */
export const setGlobalApiBase = (apiBase) => (GLOBAL_API_BASE = apiBase);
/**
 * @template Req
 * @template Res
 * @param {RequestType} reqTy
 * @param {Method} method
 * @param {string} path
 * @param {ResponseType} resTy
 * @returns {(req: Req, options?: ApiOptions) => {    data: Promise<Res>;    abort: () => void;}}
 */
const request = (reqTy, method, path, resTy) => (req, options) => {
  const controller = new AbortController();
  try {
    const promise = fetch(`${getApiBase(options)}${path}`, {
      method,
      headers:
        reqTy == "json" ? { "Content-Type": "application/json" } : void 0,
      body: reqTy == "json" ? JSON.stringify(req) : void 0,
      signal: controller.signal,
    });
    return {
      data: (async () => {
        const res = await promise;
        if (!res.ok) throw new Error(await res.text());
        if (resTy == "none") return "";
        if (resTy == "json") return await res.json();
        if (resTy == "text") return await res.text();
        throw new Error(`Unknown response type ${resTy}`);
      })(),
      abort: () => controller.abort(),
    };
  } catch (e) {
    console.error(e);
    return {
      data: Promise.reject(e),
      abort: () => controller.abort(),
    };
  }
};
/**
 * @template T
 * @template P
 * @param {(params: P) => string} url
 * @param {ResponseType} resTy
 * @returns {(params: P, options?: ApiOptions) => { cancel: () => void; listen: (stream: SSEStream<T>) => void; }}
 */
const sse = (url, resTy) => (params, options) => {
  const source = new EventSource(`${getApiBase(options)}${url(params)}`);
  /** @type {SSEStream<T> | null} */
  let stream = null;
  source.onmessage = (event) => {
    const data = event.data;
    if (resTy == "text") {
      stream?.({ type: "message", data });
    } else if (resTy == "json") {
      stream?.({ type: "message", data: JSON.parse(data) });
    } else {
      throw new Error(`Unknown response type: ${resTy}`);
    }
  };
  source.onopen = (event) => {
    stream?.({ type: "open", event });
  };
  source.onerror = (event) => {
    stream?.({ type: "error", event });
  };
  return {
    cancel: () => source.close(),
    listen: (newStream) => (stream = newStream),
  };
};
/**
 * @typedef {Object} ApiOptions
 * @property {typeof fetch} [fetch]
 * @property {string} [apiBase]
 * @property {Record<string, string>} [headers]
 */
/** @typedef {"none" | "json"} RequestType */
/** @typedef {"none" | "text" | "json"} ResponseType */
/** @typedef {"DELETE" | "GET" | "PUT" | "POST" | "HEAD" | "TRACE" | "PATCH"} Method */
/**
 * @typedef {(
 *   event:
 *     | { type: "message"; data: T }
 *     | {
 *         type: "open";
 *         event: Event;
 *       }
 *     | {
 *         type: "error";
 *         event: Event;
 *       }
 * ) => void} SSEStream
 * @template T
 */
