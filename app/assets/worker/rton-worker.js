const hardwareConcurrency = Math.max(1, self.navigator?.hardwareConcurrency ?? 1);
const requestedThreadCount = Math.max(1, Math.min(8, hardwareConcurrency - 1));
const canUseSharedThreads =
  self.crossOriginIsolated === true &&
  typeof SharedArrayBuffer !== "undefined" &&
  hardwareConcurrency > 1;

let backend = "single";
let runtimeThreadCount = 1;
let fallbackReason = canUseSharedThreads
  ? ""
  : `isolation=${self.crossOriginIsolated === true};sab=${typeof SharedArrayBuffer !== "undefined"};cores=${hardwareConcurrency}`;

const ready = (async () => {
  if (canUseSharedThreads) {
    try {
      const threaded = await import("./threaded/rton_editor_worker.js");
      await threaded.default();
      await threaded.initThreadPool(requestedThreadCount);
      backend = "threaded";
      runtimeThreadCount = Number(threaded.rton_worker_runtime_info().thread_count);
      console.info(`[RTON Worker] backend=${backend} threads=${runtimeThreadCount}`);
      return threaded;
    } catch (error) {
      fallbackReason = error instanceof Error ? error.message : String(error);
      console.warn("Threaded WASM worker initialization failed; using the single-thread backend.", error);
    }
  }

  const single = await import("./single/rton_editor_worker.js");
  await single.default();
  runtimeThreadCount = Number(single.rton_worker_runtime_info().thread_count);
  console.info(`[RTON Worker] backend=${backend} threads=${runtimeThreadCount}`);
  return single;
})();

self.onmessage = async (event) => {
  const id = event.data.id;

  try {
    const runtime = await ready;
    let response;
    if (event.data.kind === "parse") {
      response = runtime.rton_worker_parse(event.data.request);
    } else if (event.data.kind === "rton-size") {
      response = runtime.rton_worker_rton_size(event.data.request);
    } else if (event.data.kind === "text-surface") {
      response = runtime.rton_worker_text_surface(event.data.request);
    } else if (event.data.kind === "text-surface-file") {
      const buffer = await event.data.file.arrayBuffer();
      response = runtime.rton_worker_text_surface({
        bytes: new Uint8Array(buffer),
        format: event.data.format,
      });
    } else if (event.data.kind === "open-text") {
      response = runtime.rton_worker_open_text(event.data.request);
    } else if (event.data.kind === "open-text-file") {
      const buffer = await event.data.file.arrayBuffer();
      response = runtime.rton_worker_open_text({
        bytes: new Uint8Array(buffer),
        format: event.data.format,
        search_query: event.data.search_query ?? "",
      });
    } else if (event.data.kind === "tree") {
      response = runtime.rton_worker_tree(event.data.request);
    } else if (event.data.kind === "value-search") {
      response = runtime.rton_worker_value_search(event.data.request);
    } else if (event.data.kind === "locate-text") {
      response = runtime.rton_worker_locate_text(event.data.request);
    } else if (event.data.kind === "text-search") {
      response = runtime.rton_worker_text_search(event.data.request);
    } else if (event.data.kind === "hex-search") {
      response = runtime.rton_worker_hex_search(event.data.request);
    } else if (event.data.kind === "release-document") {
      response = runtime.rton_worker_release_document(event.data.request);
    } else if (event.data.kind === "batch") {
      response = runtime.rton_worker_batch(event.data.request);
    } else {
      response = runtime.rton_worker_mode_switch(event.data.request);
    }
    const rtonBytes = response?.surface?.RtonBytes;
    const transfer = [];
    if (rtonBytes instanceof Uint8Array) transfer.push(rtonBytes.buffer);
    for (const item of response?.results ?? []) {
      if (item.bytes instanceof Uint8Array) transfer.push(item.bytes.buffer);
    }
    self.postMessage({
      id,
      ok: true,
      response,
      backend,
      threadCount: runtimeThreadCount,
      fallbackReason,
    }, transfer);
  } catch (error) {
    self.postMessage({
      id,
      ok: false,
      backend,
      threadCount: runtimeThreadCount,
      fallbackReason,
      error: error instanceof Error ? error.message : String(error),
    });
  }
};
