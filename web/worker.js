const state = {
  canvas: null,
  wasm: null,
  app: null,
  raf: null,
  pendingInputs: [],
};

const postStatus = (message) => self.postMessage({ type: 'status', payload: message });
const postError = (message, error) => {
  self.postMessage({ type: 'error', payload: message });
  if (error) {
    console.error('[worker]', message, error);
  }
};

function stopLoop() {
  if (state.raf !== null) {
    cancelAnimationFrame(state.raf);
    state.raf = null;
  }
}

function forwardPointerPayload(payload) {
  if (!payload) {
    return;
  }

  if (state.app) {
    try {
      state.app.handle_pointer_event(payload);
    } catch (error) {
      postError('Worker failed to deliver pointer event', error);
    }
    return;
  }

  state.pendingInputs.push(payload);
}

async function initialiseWasm() {
  if (!state.canvas) {
    throw new Error('OffscreenCanvas missing – cannot initialise Bevy wasm.');
  }

  postStatus('Initialising wasm module…');

  try {
    const wasmModule = await import('./pkg/kingshot_pwa.js');
    if (typeof wasmModule?.default === 'function') {
      await wasmModule.default();
    }

    const { KingshotApp } = wasmModule;
    if (typeof KingshotApp !== 'function') {
      throw new Error('KingshotApp export missing');
    }

    const size = state.resize ?? { width: 1280, height: 720, devicePixelRatio: 1 };

    state.app = new KingshotApp(
      state.canvas,
      size.width,
      size.height,
      size.devicePixelRatio ?? 1
    );

    const loop = () => {
      try {
        state.app.update();
      } catch (error) {
        stopLoop();
        throw error;
      }
      state.raf = self.requestAnimationFrame(loop);
    };

    loop();

    if (state.pendingInputs.length > 0) {
      const buffer = state.pendingInputs.splice(0);
      for (const payload of buffer) {
        forwardPointerPayload(payload);
      }
    }
    postStatus('Running in dedicated worker.');
  } catch (error) {
    stopLoop();
    postError('Worker failed to start Bevy wasm', error);
  }
}

function updateResize(payload) {
  state.resize = payload;
  if (state.app) {
    state.app.resize(payload.width, payload.height, payload.devicePixelRatio ?? 1);
  }
}

self.addEventListener('message', (event) => {
  const { type, payload } = event.data ?? {};

  switch (type) {
    case 'init': {
      state.canvas = payload.canvas;
      updateResize(payload.size);
      initialiseWasm();
      break;
    }
    case 'resize': {
      updateResize(payload);
      break;
    }
    case 'input': {
      forwardPointerPayload(payload);
      break;
    }
    default:
      console.warn('[worker] Unhandled message', type, payload);
  }
});

self.addEventListener('unhandledrejection', (event) => {
  postError('Unhandled rejection in worker', event.reason);
});

self.addEventListener('error', (event) => {
  postError('Worker runtime error', event.error ?? event.message);
});
