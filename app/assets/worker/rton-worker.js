import init, {
  rton_worker_mode_switch,
  rton_worker_open_text,
  rton_worker_parse,
  rton_worker_rton_size,
  rton_worker_text_surface,
} from "./rton_editor_worker.js";

const ready = init();

self.onmessage = async (event) => {
  await ready;

  try {
    let response;
    if (event.data.kind === "parse") {
      response = rton_worker_parse(event.data.request);
    } else if (event.data.kind === "rton-size") {
      response = rton_worker_rton_size(event.data.request);
    } else if (event.data.kind === "text-surface") {
      response = rton_worker_text_surface(event.data.request);
    } else if (event.data.kind === "text-surface-file") {
      const buffer = await event.data.file.arrayBuffer();
      response = rton_worker_text_surface({
        bytes: new Uint8Array(buffer),
        format: event.data.format,
      });
    } else if (event.data.kind === "open-text") {
      response = rton_worker_open_text(event.data.request);
    } else if (event.data.kind === "open-text-file") {
      const buffer = await event.data.file.arrayBuffer();
      response = rton_worker_open_text({
        bytes: new Uint8Array(buffer),
        format: event.data.format,
        search_query: event.data.search_query ?? "",
      });
    } else {
      response = rton_worker_mode_switch(event.data.request);
    }
    self.postMessage({ ok: true, response });
  } catch (error) {
    self.postMessage({
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
};
