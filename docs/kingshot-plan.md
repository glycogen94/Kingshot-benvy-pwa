# Kingshot Mobile Defense Adaptation Plan

## Vision & Experience Goals
- Deliver a touch-first, hero-centric defense experience inspired by the mobile game "Kingshot" while leveraging Bevy + WebGPU in the browser.
- Match native mobile feel with high-DPI rendering, fluid animations, responsive gestures, and low input latency.
- Support both worker-based and main-thread execution paths without diverging gameplay logic.

## Reference Game Research (Century Games' Kingshot)
- Publisher: Century Games (Sentry Games label), global release on iOS/Android with SLG tower-defense hybrid focus.
- Narrative premise: governors rebuild a collapsed kingdom—rescue refugees, restore town center, manage resources, repel rebel invasions.
- Core systems: town-building economy, hero recruitment/upgrade, alliance warfare, PvE wave defense, asynchronous PvP for rankings.
- Progression loops: idle timers for production, technology tree, law system, wave/boss events, seasonal live-ops (per marketing site + store listing).
- Art & presentation: painterly medieval fantasy with 3D-rendered character portraits and high-contrast UI (gold/emerald palette), cinematic splash art, isometric battlefield shots.
- Monetization hooks: in-app purchases, gacha-like hero roster, speed-ups, cosmetics per store positioning.
- UX patterns: vertical orientation, dense HUD with radial ability buttons, floating call-to-action prompts, high readability text overlays over blurred backgrounds.
- Differentiators to emulate: dramatic invasion set pieces, hero-focused storytelling, cooperative alliance content, animated town life, frequent live events.
- Pain points (from reviews/trends): heavy monetization expectations, grind for resources, reliance on idle timers—consider softer pacing for our adaptation.

## Core Gameplay Pillars
- **Hero Defense Loop**: Player defends a castle by slinging projectiles at incoming enemies, chaining shots for combos.
- **Lane Pressure Management**: Multiple enemy lanes advance toward the castle; the player prioritises threats via quick target selection.
- **Ability Bursts**: Cooldown-based hero skills (e.g., multishot, explosive arrow) provide clutch crowd control moments.
- **Progression & Rewards**: Session-based XP, unlockable gear/abilities, and escalating difficulty via waves/bosses.

## High-Level Architecture
- `app` (existing `src/lib.rs`): orchestrates Bevy app, window config, and shared resources.
- `game` module: owns game states (`Loading`, `MainMenu`, `Gameplay`, `Pause`, `GameOver`) and transitions using `bevy::app::AppStates`.
- `loading` module: asynchronous asset pipeline, handles preloading textures, audio, and level data; emits progress events for UI.
- `level` module: wave definitions, spawn schedules, lane paths, and difficulty curves. Emits `SpawnRequest` events consumed by `units`.
- `units` module: ECS components for enemies, towers/projectiles, health, behaviours. Splits into sub-plugins (`enemy`, `projectile`, `effects`).
- `combat` module: hit detection, damage resolution, particle/VFX triggers, combo scoring.
- `input` module: touch/gesture translation and action mapping (tap, drag, flick, pinch). Produces semantic input events consumed by gameplay systems.
- `camera` module: smooth follow/zoom behaviour with screen shake support. Reacts to `input` gestures and device orientation changes.
- `ui` module: HUD overlays (health, cooldowns), menus, tutorial prompts built with `bevy_ui` + `bevy_tweening`.
- `audio` module: background music, SFX routing, gesture-triggered audio cues with rate limiting on wasm.
- `persistence` module (later milestone): localStorage-backed profile, settings, and progression.

Each module exposes a `Plugin` for clean registration from `build_app`. Cross-cutting data flows through typed events/resources to keep worker boundaries deterministic.

## Touch & Gesture Strategy
- Capture pointer/touch events on the DOM canvas (`pointerdown/up/move`, `wheel`, `gesturechange` fallback) and forward to the worker via postMessage (`{ type: 'input', payload }`).
- In the main-thread path, mirror the same event payloads and inject them into Bevy via `EventWriter<RawTouchEvent>` to keep behaviour consistent.
- Maintain a touch session map (touch id → start position/time) to derive:
  - **Tap / Multi-tap**: quick press with minimal movement.
  - **Drag**: continuous move updates for aim/lining shots.
  - **Flick / Swipe**: high-velocity release triggers special shots.
  - **Pinch / Spread**: interval-based zoom control for camera.
- Convert gesture detections into action enums (`InputAction::Aim`, `InputAction::CastAbility(AbilitySlot)`, `InputAction::Zoom { delta }`).
- Provide adaptive touch targets and UI scaling using `CanvasSize` resource for high-DPI devices.

## Gameplay Loop Outline
1. **Loading State**: Preload atlases, meshes, sounds; display animated progress overlay.
2. **Main Menu State**: Present start button, settings, ability loadout; respond to app install prompts (PWA).
3. **Gameplay State**:
   - Spawn lanes and initial wave.
   - Process `InputAction` events to control aiming reticle and ability casts.
   - Update enemy AI along spline paths, check castle proximity.
   - Simulate projectile physics, resolve collisions, update combo meters.
   - Trigger VFX/audio, update HUD.
4. **Pause/Game Over States**: Freeze simulation, show overlays with stats, allow restart.

## Asset & Rendering Considerations
- Use texture atlases or sprite sheets sized for 2048x2048 limit; rely on `bevy_ecs_tilemap` or custom quad batching for performance.
- Implement dynamic resolution scaling option; adjust render scale for low-end devices.
- Queue GPU-friendly particle effects (GPU instancing) for impact feedback.

## Worker Communication Extensions
- Extend worker protocol with messages:
  - `{ type: 'input', payload: PointerEventPayload }`
  - `{ type: 'haptics', payload: { pattern } }` (future iOS-style feedback via Web API).
  - `{ type: 'app_state', payload: { state } }` for debugging overlays.
- Ensure messages remain serialisable (avoid closures/Map). Document schema in `docs/kingshot-plan.md` for DOM ↔ wasm integration.

### PointerEventPayload (DOM → wasm)
| Field | Type | Notes |
| --- | --- | --- |
| `phase` | string | `'start'`, `'move'`, `'end'`, `'cancel'` mapped from pointer lifecycle. |
| `pointerId` | number | Native `PointerEvent.pointerId` for multi-touch tracking. |
| `pointerType` | string | `'touch'`, `'pen'`, `'mouse'`; used to prioritise gesture heuristics. |
| `buttons` | number | Bitmask from `PointerEvent.buttons`; zero on touch end. |
| `clientX`/`clientY` | number | Browser client-space coordinates (CSS px) for analytics/debug. |
| `canvasX`/`canvasY` | number | Canvas-relative logical coordinates (0..canvas width/height). |
| `normalizedX`/`normalizedY` | number | Canvas-relative values normalised to 0.0–1.0 for DPI-independent logic. |
| `pressure` | number | Range 0.0–1.0; defaults to `event.pressure || (buttons ? 0.5 : 0)`. |
| `timestamp` | number | `performance.now()` in milliseconds for gesture velocity. |
| `modifiers` | object | `{ alt, ctrl, meta, shift }` booleans. |
| `deltaX`/`deltaY` | number | Optional; populated when `phase === 'move'` for pointer delta (CSS px). |
| `wheelDelta` | number | Optional; derived from `WheelEvent.deltaY` when originating from wheel/trackpad. |

`PointerEventPayload` will be forwarded via `postMessage` to the worker or invoked directly on the main-thread `KingshotApp`, ensuring the Bevy input layer receives identical data regardless of execution mode.

## Testing & Validation
- Add wasm-bindgen integration test covering `input` event ingestion and state transitions for the first wave.
- Provide deterministic replay harness for gesture streams (recorded pointer events) to validate combos across builds.
- For touch logic, include headless tests that feed synthetic gesture trajectories into the interpreter.

## Immediate Implementation Tasks
- [x] Replace `scene::DemoScenePlugin` with foundational `core` and `game` plugins that set up app states/resources.
- [x] Extend worker + main-thread scripts to forward pointer/touch payloads into Bevy (`InputAction` event channel).
- [x] Prototype first wave: hero entity, basic enemy lane, projectile firing using placeholder meshes/audio.

## Follow-Up Backlog
- Ability HUD and cooldown UI overlay with responsive layout.
- Gesture tutorial/onboarding sequence tied to early waves.
- Audio layering, dynamic music stingers, and haptic feedback bridge.
- Progression persistence (localStorage) with upgrade tree UX.
- Performance pass: batch rendering, pooling projectiles, dynamic resolution scaling toggles.
- Replace temporary auto-advance timers with gesture-driven menu selections once input channel lands.
- Enrich wave controller (spawn pacing, enemy variety) and expand pointer actions beyond tap-to-fire.

## Next Steps
- Build the HUD overlay to communicate health, wave progress, and ability cooldowns.
- Extend the input interpreter with swipe/pinch gestures and tie them to prototype hero abilities.
- Introduce a wave scheduler resource to pace enemy spawns and surface combo/score feedback in the UI.

Backlog items and milestone updates should continue to append to this document as functionality lands.
