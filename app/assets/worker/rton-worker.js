import init, {
  rton_worker_mode_switch,
  rton_worker_parse,
  rton_worker_rton_size,
} from "./rton_editor_worker.js";

const ready = init();

self.onmessage = async (event) => {
  await ready;

  try {
    const response =
      event.data.kind === "parse"
        ? rton_worker_parse(event.data.request)
        : event.data.kind === "rton-size"
          ? rton_worker_rton_size(event.data.request)
          : rton_worker_mode_switch(event.data.request);
    self.postMessage({ ok: true, response });
  } catch (error) {
    self.postMessage({
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
};
