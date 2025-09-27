const STATUS = document.querySelector('#status');
const CANVAS = document.querySelector('#game-canvas');

const SUPPORTS_OFFSCREEN = typeof OffscreenCanvas !== 'undefined' && typeof HTMLCanvasElement !== 'undefined';
const SUPPORTS_WORKERS = typeof Worker !== 'undefined';
const SUPPORTS_WEBGPU = navigator.gpu !== undefined;

const query = new URLSearchParams(window.location.search);
const ENABLE_WORKER = window.__kingshot_enableWorker === true || query.get('worker') === '1';

const state = {
  worker: null,
  resizeObserver: null,
  raf: null,
  mainApp: null,
  detachInput: null,
};

const pointerCache = new Map();

if (CANVAS) {
  CANVAS.style.touchAction = 'none';
  CANVAS.addEventListener('contextmenu', (event) => event.preventDefault());
}

const updateStatus = (text) => {
  if (STATUS) {
    STATUS.textContent = text;
  }
};

async function registerServiceWorker() {
  if ('serviceWorker' in navigator) {
    try {
      await navigator.serviceWorker.register('/service-worker.js', { scope: '/' });
      console.debug('[pwa] service worker registered');
    } catch (error) {
      console.warn('[pwa] service worker registration failed', error);
    }
  }
}

function calculateCanvasSize() {
  const rect = CANVAS.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  return {
    width: Math.max(1, Math.floor(rect.width * dpr)),
    height: Math.max(1, Math.floor(rect.height * dpr)),
    logicalWidth: rect.width,
    logicalHeight: rect.height,
    devicePixelRatio: dpr,
  };
}

function watchResize(callback) {
  if (state.resizeObserver) {
    state.resizeObserver.disconnect();
  }

  const handler = () => callback(calculateCanvasSize());

  const observer = new ResizeObserver(handler);
  observer.observe(CANVAS);
  window.addEventListener('resize', handler, { passive: true });
  state.resizeObserver = observer;

  return () => {
    observer.disconnect();
    window.removeEventListener('resize', handler);
  };
}

function teardownWorker() {
  if (state.worker) {
    state.worker.terminate();
    state.worker = null;
  }
  if (state.resizeObserver) {
    state.resizeObserver.disconnect();
    state.resizeObserver = null;
  }
}

function startWorker(offscreenCanvas) {
  const worker = new Worker('./worker.js', { type: 'module' });

  worker.addEventListener('message', (event) => {
    const { type, payload } = event.data ?? {};
    switch (type) {
      case 'status':
        updateStatus(payload ?? '');
        break;
      case 'error':
        console.error('[worker]', payload);
        updateStatus('Worker error – reload without ?worker=1 to run on main thread.');
        break;
      default:
        console.debug('[worker message]', event.data);
    }
  });

  const initialSize = calculateCanvasSize();
  worker.postMessage(
    {
      type: 'init',
      payload: {
        canvas: offscreenCanvas,
        size: initialSize,
      },
    },
    [offscreenCanvas]
  );

  watchResize((size) => {
    worker.postMessage({ type: 'resize', payload: size });
  });

  state.worker = worker;
  ensurePointerBridge();
}

async function bootstrapOnMainThread() {
  updateStatus('Initialising on main thread…');

  const module = await import('./pkg/kingshot_pwa.js');
  if (typeof module?.default === 'function') {
    await module.default();
  }

  const { KingshotApp } = module;
  if (typeof KingshotApp !== 'function') {
    throw new Error('KingshotApp export missing');
  }

  if (!SUPPORTS_OFFSCREEN) {
    throw new Error('OffscreenCanvas not supported in this browser.');
  }

  const size = calculateCanvasSize();
  const offscreenCanvas = CANVAS.transferControlToOffscreen();

  state.mainApp = new KingshotApp(
    offscreenCanvas,
    size.width,
    size.height,
    size.devicePixelRatio
  );

  const frame = () => {
    try {
      state.mainApp.update();
    } catch (error) {
      cancelAnimationFrame(state.raf);
      throw error;
    }
    state.raf = requestAnimationFrame(frame);
  };

  frame();
  updateStatus('Running on main thread.');

  watchResize((next) => {
    state.mainApp.resize(next.width, next.height, next.devicePixelRatio);
  });

  ensurePointerBridge();
}

async function bootstrap() {
  await registerServiceWorker();

  if (!SUPPORTS_WEBGPU) {
    updateStatus('WebGPU not available – enable it in browser flags or upgrade.');
    return;
  }

  if (ENABLE_WORKER) {
    if (!SUPPORTS_WORKERS || !SUPPORTS_OFFSCREEN) {
      console.warn('[main] Worker path requested but unsupported, falling back to main thread.');
      await bootstrapOnMainThread();
      return;
    }

    updateStatus('Spawning worker…');
    const offscreenCanvas = CANVAS.transferControlToOffscreen();
    startWorker(offscreenCanvas);
    return;
  }

  await bootstrapOnMainThread();
}

bootstrap().catch((error) => {
  teardownWorker();
  console.error('[main] bootstrap failed', error);
  updateStatus('Fatal error during bootstrap');
});

function ensurePointerBridge() {
  if (!CANVAS || state.detachInput) {
    return;
  }

  const sendPayload = (payload) => {
    if (!payload) {
      return;
    }

    if (state.worker) {
      state.worker.postMessage({ type: 'input', payload });
      return;
    }

    if (state.mainApp && typeof state.mainApp.handle_pointer_event === 'function') {
      try {
        state.mainApp.handle_pointer_event(payload);
      } catch (error) {
        console.error('[input] failed to forward pointer payload', error);
      }
    }
  };

  const updateCache = (pointerId, canvasX, canvasY) => {
    pointerCache.set(pointerId, { x: canvasX, y: canvasY });
  };

  const removeFromCache = (pointerId) => {
    pointerCache.delete(pointerId);
  };

  const buildPayload = (event, phase) => {
    const rect = CANVAS.getBoundingClientRect();
    const canvasX = event.clientX - rect.left;
    const canvasY = event.clientY - rect.top;
    const width = rect.width || 1;
    const height = rect.height || 1;
    const pointerId = event.pointerId ?? 0;

    let deltaX = 0;
    let deltaY = 0;

    if (phase === 'move') {
      const previous = pointerCache.get(pointerId);
      if (previous) {
        deltaX = canvasX - previous.x;
        deltaY = canvasY - previous.y;
      }
    }

    if (phase === 'start' || phase === 'move') {
      updateCache(pointerId, canvasX, canvasY);
    } else {
      removeFromCache(pointerId);
    }

    const normalizedX = Math.min(Math.max(canvasX / width, 0), 1);
    const normalizedY = Math.min(Math.max(canvasY / height, 0), 1);

    const pressure =
      typeof event.pressure === 'number' && Number.isFinite(event.pressure)
        ? event.pressure
        : event.buttons
        ? 0.5
        : 0;

    return {
      phase,
      pointerId,
      pointerType: event.pointerType || 'unknown',
      buttons: event.buttons ?? 0,
      clientX: event.clientX,
      clientY: event.clientY,
      canvasX,
      canvasY,
      normalizedX,
      normalizedY,
      pressure,
      timestamp: performance.now(),
      modifiers: {
        alt: !!event.altKey,
        ctrl: !!event.ctrlKey,
        meta: !!event.metaKey,
        shift: !!event.shiftKey,
      },
      deltaX,
      deltaY,
      wheelDelta: 0,
    };
  };

  const handlePointerDown = (event) => {
    if (event.target !== CANVAS) {
      return;
    }
    event.preventDefault();
    try {
      CANVAS.setPointerCapture(event.pointerId);
    } catch (error) {
      console.warn('[input] failed to capture pointer', error);
    }
    sendPayload(buildPayload(event, 'start'));
  };

  const handlePointerMove = (event) => {
    if (!pointerCache.has(event.pointerId)) {
      return;
    }
    event.preventDefault();
    sendPayload(buildPayload(event, 'move'));
  };

  const handlePointerEnd = (event) => {
    if (!pointerCache.has(event.pointerId)) {
      return;
    }
    event.preventDefault();
    try {
      if (CANVAS.hasPointerCapture(event.pointerId)) {
        CANVAS.releasePointerCapture(event.pointerId);
      }
    } catch (error) {
      console.warn('[input] failed to release pointer', error);
    }
    sendPayload(buildPayload(event, 'end'));
  };

  const handlePointerCancel = (event) => {
    if (!pointerCache.has(event.pointerId)) {
      return;
    }
    event.preventDefault();
    try {
      if (CANVAS.hasPointerCapture(event.pointerId)) {
        CANVAS.releasePointerCapture(event.pointerId);
      }
    } catch (error) {
      console.warn('[input] failed to release pointer on cancel', error);
    }
    sendPayload(buildPayload(event, 'cancel'));
  };

  CANVAS.addEventListener('pointerdown', handlePointerDown, { passive: false });
  CANVAS.addEventListener('pointermove', handlePointerMove, { passive: false });
  CANVAS.addEventListener('pointerup', handlePointerEnd, { passive: false });
  CANVAS.addEventListener('pointercancel', handlePointerCancel, { passive: false });
  CANVAS.addEventListener('pointerleave', handlePointerCancel, { passive: false });

  state.detachInput = () => {
    CANVAS.removeEventListener('pointerdown', handlePointerDown);
    CANVAS.removeEventListener('pointermove', handlePointerMove);
    CANVAS.removeEventListener('pointerup', handlePointerEnd);
    CANVAS.removeEventListener('pointercancel', handlePointerCancel);
    CANVAS.removeEventListener('pointerleave', handlePointerCancel);
    pointerCache.clear();
    state.detachInput = null;
  };
}
