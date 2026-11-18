# Classic EXP Tracker — Base Context

Living document. Decisions are added as they are made; open items live in [Open questions](#open-questions).

## 1. Goal

Measure training efficiency in **MapleStory Classic** while playing normally.

The core metric is the **EXP rate**:

```
EXP rate = EXP gained / elapsed time
```

Reported as **EXP/min and EXP/h, both always visible**. No other units.

## 2. Hard constraints

| Constraint | Detail |
|---|---|
| Screen reading only | The tool only reads pixels from the screen. No memory reading, no process injection, no packet inspection, no input automation. |
| Non-intrusive | Must not interfere with the game client in any way. Passive observer. |
| No in-game overlays | Nothing is drawn into or hooked onto the game window. All UI lives in the tracker's own, separate OS windows. |
| Executable | Ships as a native executable, no runtime to install. |
| Cross-platform | macOS and Windows. |
| Local first | All data stays on the machine for now. A remote service may come later. |
| Performance | Minimal CPU/GPU/RAM footprint. The game must not lose frames because of the tracker. |

## 3. Input source

MapleStory Classic shows an always-visible HUD with the player's **EXP as both a raw number and a percentage**, plus the character **level** and **name**. These are the only data sources.

The tracker captures **only those regions** of the game window, never the full frame.

### 3.0 Status bar layout (confirmed from a real capture, 2026-11-15)

Reference capture: macOS, windowed, window content 1920×1228 px including the ~28 px macOS title bar. All fields live in the **status bar anchored to the bottom edge** of the window.

```
LV. [3][5]   Spearman      HP[1758/1758]   MP[722/722]   EXP. 54,244 [31.14%]
             <name>        (red bar)       (blue bar)    (yellow bar)
```

| Field | Rendering | Approx. region in reference capture (x, y, w, h) |
|---|---|---|
| Level | White digits, each inside its own orange tile, after a white `LV.` label | 606, 1204, 23, 13 |
| Job | White pixel font, first line next to the level | 643, 1196, 70, 13 |
| Character name | White pixel font, second line | 643, 1209, 70, 13 |
| EXP | `EXP.` label, then current EXP with `,` thousands separator, then percent with 2 decimals in green brackets: `54,244 [31.14%]` | 1023, 1195, 106, 15 |

Observations:
- The EXP value is the **current EXP within the level**, not `current / required`. Required EXP is inferred: `54244 / 0.3114 ≈ 174,194`.
- Fonts are crisp pixel fonts (no anti-aliasing visible at 1:1), white on a dark gray bar: ideal for glyph template matching.
- Level digits use a different glyph set (white on orange tiles) than the EXP digits, so they need their own templates.
- Name and job sit at fixed positions, so the name can be read with the same pipeline (larger alphabet) or with the general OCR fallback.
- `EXP.` and `LV.` labels are stable anchors for auto-detecting the regions.
- Regions are relative to the bottom-left of the window; they must be re-derived when the window size changes.

### 3.1 Game client environments

| Platform | How the game runs | Capture implication |
|---|---|---|
| Windows | Native client | Capture by window |
| macOS | Separate wrapped process (not native, but close) | Capture by window via ScreenCaptureKit; the window belongs to the wrapper process |

On macOS the client runs under Wine from the launcher bundle; the game process is named `MapleStory Classic`. Window lookup matches `maplestory` in app name or title and skips the launcher.

### 3.1.1 Screen permission

- **macOS:** requires Screen Recording permission. Without it the OS only exposes system windows (menu bars), so the game window cannot even be listed. Checked with `CGPreflightScreenCaptureAccess`, requested with `CGRequestScreenCaptureAccess` (the system prompt appears only once; afterwards the user toggles it in System Settings). With `pnpm tauri dev` the permission belongs to the app that launched the tracker (terminal, IDE, Claude), not to the tracker. To grant it to the tracker itself, run it as its own bundle with `pnpm app:mac` (builds a debug `.app`, signs it, opens it through LaunchServices).
- Signing uses the private key in the login keychain. To stop the keychain password prompt on every rebuild, run `pnpm fix:signing` once: it adds `codesign` to the key's partition list (`security set-key-partition-list`) and asks for the login keychain password one time.
- The permission is tied to the code signature. An ad-hoc signature changes on every rebuild and the grant is lost; a stable identity keeps it. The identity is read from `APPLE_SIGNING_IDENTITY` in the untracked `.env.local` (see `.env.example`).
- After an app update or a signing-identity change macOS can show the permission as on while capture still fails. Fix: remove the app from the Screen Recording list with **−**, add or reopen it, and grant again. Keep the same signing identity across releases to avoid this.
- **Windows:** no permission needed.
- The main window has a **Game capture** panel: permission status, request button, open-settings button and **Detect game window**, which captures one frame, shows a preview, saves it as `captures/last-capture.png` in the app-data directory and says whether the status bar was found and at which scale.

### 3.2 Display modes

- **Windowed: primary target.** Most common for the user and the safest setup.
- Borderless / fullscreen: must also work. Exclusive fullscreen may not be capturable by window on Windows; borderless is expected to work.

## 4. Technology decisions

### 4.1 Language: Rust — CONFIRMED

Rust fits every constraint: single native binary per platform, no GC pauses, low memory, first-class bindings to the OS capture APIs on both platforms.

### 4.2 Screen capture — PROPOSED

| Platform | API | Crate candidates |
|---|---|---|
| Windows | Windows Graphics Capture (WGC) | `windows-capture`, `xcap` |
| macOS | ScreenCaptureKit | `screencapturekit`, `xcap` |

Strategy:
- Capture the game **window** (not the whole desktop) and crop to the regions of interest (ROI).
- Low sample rate (e.g. 1–2 Hz is enough: EXP only changes on kills/quests). Configurable.
- Skip OCR entirely when the ROI pixels did not change since the last frame (cheap hash compare).

### 4.3 ROI selection — CONFIRMED (both modes)

- **Auto:** locate the HUD inside the game window (anchor on stable HUD features such as the EXP bar shape/colors).
- **Manual:** the user drags a rectangle over each region once; stored per window size.
- Auto runs first; manual overrides it.

### 4.4 OCR — PROPOSED

The EXP and level fields use a **fixed game font**, at a fixed size, on a mostly fixed background, with a tiny alphabet (`0-9`, `.`, `%`, `[`, `]`, `/`, `,`). General-purpose OCR is slower and *less* reliable than a specialized reader for this case. The character name is the exception: free text with a larger alphabet, but it only needs to be read once per session.

Options evaluated:

| Option | Accuracy on game HUD digits | CPU cost | Binary/deps | Notes |
|---|---|---|---|---|
| **Glyph template matching (custom, pure Rust)** | Near 100% once glyphs are calibrated (pixel font is deterministic) | Microseconds per read | None | Breaks only if the font/scale changes; mitigated by a calibration step |
| PaddleOCR PP-OCRv5 via ONNX Runtime (`oar-ocr`, `paddle-ocr-rs`) | Highest among general OCR engines | Milliseconds per read, ~10–20 MB models | ONNX Runtime shared lib | Best general-purpose fallback |
| `ocrs` (pure Rust, `rten` runtime) | Good on clean text, weaker on tiny pixel fonts | Milliseconds | Pure Rust | Simpler build than ONNX |
| OS-native OCR (Apple Vision / `Windows.Media.Ocr`) | Very good (Vision), good (Windows) | Low, OS-accelerated | Zero bundled models | Two code paths, results differ per OS |
| Tesseract (`tesseract-rs`, `leptess`) | Weak on small game fonts without tuning | Moderate | C++ library | Not recommended |

**Recommendation:** a two-tier reader.

1. **Primary (EXP, %, level): glyph template matching.** Binarize the ROI, segment characters by column projection, match each against calibrated glyph templates. Deterministic, effectively zero CPU, no dependencies.
2. **General OCR (character name, calibration, recovery): PP-OCRv5 recognition model via ONNX Runtime** (recognition model only; the ROI is already known so no detection model is needed). Reads the name once per session, auto-generates glyph templates during calibration, and recovers if template matching confidence drops (e.g. UI scale change).

Every reading is validated before being accepted (see 5.2), so a bad OCR read never corrupts the metric.

#### Implemented reader (`crates/ocr`)

1. **Locate:** binarize the bottom 90 px of the window (pure white pixels) and slide the `EXP.` and `LV.` label templates over it (≤ 4% differing pixels). The EXP region is to the right of `EXP.`; the level region is the bounding box of the orange tiles right of `LV.`. Re-run when the window size changes or after 3 failed reads.
   - **Display scaling (`scale.rs`):** when the 1:1 match fails, the capture may be stretched by the OS (Windows scales DPI-unaware games to 125%, 150%… with bilinear blur). The reader then tries the Windows scale steps 125–300%: a coarse pass picks the scale by label match (≤ 20% differing pixels), a fine pass picks the sub-pixel offset (8×8 grid) by how many glyphs of the EXP and level lines read. The band is rebuilt at 1:1 by sampling each game pixel at its center plus a contrast stretch (40–175) that puts blurred ink back over the color thresholds. All later steps run on that rebuilt band. The found view is kept; the full search (≤ 0.5 s) runs at most every 10 s while the status bar is not found. Rebuilding the band costs ~0.7 ms per read.
   - Verified only on synthetic stretches of the reference capture: bilinear 125–300% at every sub-pixel offset read exactly; sharpened (Catmull-Rom) stretches fail at 250% and up.
2. **Binarize:** EXP font ink = white or green (brackets); level ink = pure white.
3. **Segment:** split glyphs on empty columns, trim each to its ink, and store its top relative to the tallest glyph (so `,` and `.` are told apart and ROI shifts do not matter).
4. **Match:** exact template lookup by size and shape, tolerating 1 differing pixel and ±1 px vertical offset. Unmatched glyphs become `?`.
5. **Parse:** `54,244[31.14%]` → `(54244, 31.14)`; text after `]` is ignored; a `.` in the EXP value is read as a `,` (rescaled commas can look like dots).
6. **Change detection:** an FNV hash of both regions skips reading when nothing changed.

Measured on the reference capture (release build): locate 10 ms once, read 21 µs per frame.

#### Glyph templates and self-learning

- Built-in templates (`crates/ocr/assets/*.glyphs`, plain text `#`/`.` bitmaps) were calibrated from the reference capture with `cargo run -p ocr --example calibrate`. Known so far: EXP font `0-9` minus `0 6 9`, plus `, . % [ ] /`; level font `3 5`.
- Missing EXP digits are inferred: each unknown shape is tried as every digit, and the guess is accepted only when exactly one assignment makes `exp / required ≈ percent` against the session's known required EXP. Ambiguous cases are skipped, never guessed.
- Missing level digits are learned from the EXP table (see below), or from context: previous + 1 when the percent drops by more than 50 points (level-up).
- Learned glyphs are saved to `glyphs/*.learned.glyphs` in the app-data directory and merged at startup.

#### Identity read (name + level) — once at startup

- When the app opens, the reader captures the game once and reads the **character name** and **level**; it retries every 5 s until it succeeds (game closed, permission missing, status bar hidden). A "Re-read name & level" button forces a new read.
- **Name:** read with the OS OCR engine (`crates/sysocr`): Apple Vision on macOS, `Windows.Media.Ocr` on Windows. Zero bundled models; used only for this one read, so its ~0.2–0.5 s cost does not matter. Verified on the reference capture: it returns the character name (and `Spearman` for the job line).
- **Level:** from the level glyph templates; if a digit is unknown, deduced from the EXP table: `exp × 100 / percent` matches exactly one level's requirement (reference: 54,244 / 31.14% ≈ 174,194 vs table 174,216 for level 35). The deduced level then teaches the missing level glyphs.
- The OS OCR is **not** used for the level or EXP: Apple Vision misreads the bold digits on orange tiles (`5 E` for `35`) and the bracketed EXP line.
- This replaces the earlier PP-OCRv5 fallback idea: no ONNX Runtime, no models to ship.

#### During a session: EXP only

- Each tick reads only the EXP region (hash-gated). The level is not read again: it is taken from the EXP table when exp/percent match a level uniquely, otherwise previous level + 1 on a >50-point percent drop.

### 4.4.1 EXP table — CONFIRMED

MapleStory Classic table (levels 1–99, cap 100) from public game data, embedded in `exp-core` (`ExpTable::classic()`). Used for exact required EXP, level-up gain, time to level, level deduction and EXP digit inference.

### 4.5 UI — CONFIRMED

Two separate windows owned by the tracker, never drawn inside the game:

| Window | Purpose | Behavior |
|---|---|---|
| **Compact HUD** | Glanceable live rate (EXP/h, EXP/min, time to level) | Small, borderless, semi-transparent, always-on-top, click-through optional. **May sit over a corner of the game window**: it is a separate OS window, the game is never touched |
| **Main window** | Session controls, ROI setup, history, settings | Small, regular window, low-key styling |

Both are personalized with the character **name** and **level** read from the HUD.

**Framework: Tauri v2.** Rust backend (capture, OCR, metrics, storage) + web frontend in the system webview (WebView2 on Windows, WKWebView on macOS). Multi-window, transparent and always-on-top windows are supported natively.

Performance rules for the UI layer:
- All heavy work stays in Rust; the webview only renders.
- Backend pushes events to the frontend only when a value changes (no polling from JS).
- No animations or timers running in the frontend while idle.

**Frontend: Svelte 5 + TypeScript + Vite.** Compiles to plain JS with no virtual DOM and a tiny runtime.

#### Visual style — Maple Classic

Built only from colors, shapes and type of the game's own HUD pieces (no game art or assets), and deliberately not from the game's parchment windows, so it has its own identity:

| Source in the game | Used for |
|---|---|
| Status bar steel `#5a636b → #2e353b` with `#b1c1cd` edge | Table headers, secondary buttons, square `+ / −` buttons, HUD name strip |
| Item tooltips (dark translucent glass) | Panels: night glass `rgba(24,32,48,.92) → rgba(12,17,28,.92)`, 1 px steel rim at 32% |
| Quick-slot squares | Bevelled square icon slots in each section header |
| Damage numbers | EXP/min and EXP/h: Fredoka 700, yellow `#ffe066` with a dark `#4a2000` outline |
| System chat | Gold `#ffd400` for the gain feed, AFK tag and best-session line |
| Level tiles `#ff7d12` | `LV.` tiles and the `Lv 36 in 1h 02m` line |
| EXP bar `#def700 → #a7bb00` | Only the EXP bar itself |
| HP red `#e5484d` | Behind best, stop icon |

The UI accent is ice blue `#38c8ff` (primary button, active tab, ahead-of-best tag, running dot, chart line); it pairs with the warm digits and the orange level color and never reuses the EXP lime. Ahead/behind also differ by ▲/▼ and lightness, not hue alone.

Fonts are bundled locally through `@fontsource/nunito` (latin 400–900) and `@fontsource/fredoka` (latin 600–700); no network fonts. Design tokens and the shared `.glass`, `.slot`, `.dmg`, `.tag` classes live in `src/app.css`; shared components are `LevelBadge`, `ExpBar`, `Sparkline`, `Icon` and `Accordion` in `src/lib`.

#### Browser preview

`src/lib/backend.ts` wraps every Tauri call. Outside Tauri (plain browser, `pnpm dev` at `http://127.0.0.1:1420`) it serves fake data, so the main window (`/`) and the HUD (`/hud`) can be designed and checked without the game or the native app.

#### Compact HUD content

| Field | Notes |
|---|---|
| Character name + level | Header line |
| Pace vs best | Header tag, `▲ 2m` ice blue when ahead of the best saved session, `▼ 45s` red when behind; `AFK` gold when auto-paused |
| **EXP/min and EXP/h** | Side by side, for the chosen rate window |
| Rate chart | Own row between the rates and the EXP bar (never under the numbers), best-session rate as a dashed gold line. Toggled by the chart button in the name strip or from the main window; with the chart the HUD box is 242×115, without it 242×97 |
| EXP gained | Total this session |
| Session time | Active time only |
| Time to next level | `Lv 36 in 11m`, at the chosen window's rate |
| Level progress % | Thin bar |
| Session state | Dot: running (green), paused (gold), auto-paused (blinking amber); auto-pause or reader warning line |
| Hover controls | Start/pause and stop & save buttons, shown on hover while the HUD is unlocked. The chart button is always shown; it only takes clicks while unlocked (locked is click-through) |
| Level up | Gold glow and a `LEVEL UP!` burst for 2.6 s |

#### Main window content

Always visible: name, level, state and session clock (with a level-up glow); rate-window tabs `1 min | 5 min | 15 min | Session`; EXP/min and EXP/h for the chosen window; gap to the best session (`▲ 1m 36s AHEAD OF YOUR BEST`, or "No best yet at this level"); rate chart with the best-session line and a show/hide button (remembered per machine); level progress; `Lv 36 in 1h 02m` and EXP to go; feed of the last 3 EXP gains with their time; session buttons.

Collapsed sections show a one-line summary (HUD settings, reader status, saved sessions and best, first shortcuts).

Collapsible sections (open/closed state remembered per section):

| Section | Default | Content |
|---|---|---|
| Session | Open | EXP gained, session time, EXP to level, levels gained; EXP/min and EXP/h for last 1, 5, 15 min and session; auto-pause delay |
| HUD | Closed | Content size 100–200%, background opacity 30–100% (slider and −/+ each), show/hide, lock position |
| Game capture | Closed | Screen permission, game window detection with preview, reader status, re-read name & level |
| History | Closed | Last 10 saved sessions |
| Shortcuts | Closed | Global hotkeys |
| Developer | Closed, debug builds only | Fake reading feed to test the UI without the game |

#### Settings

Stored as `settings.json` in the app-data directory, read at startup, changed through `update_settings` (partial patch, clamped, saved, pushed to both windows as a `settings` event).

| Key | Default | Range |
|---|---|---|
| `hudScale` | 1.0 | 1.0–2.0 |
| `hudChart` | true | Show the rate chart row in the HUD |
| `hudOpacity` | 0.9 | 0.3–1.0, background only; text stays opaque |
| `rateWindow` | `5m` | `1m`, `5m`, `15m`, `session`; drives the headline rates and time to level |
| `idlePauseMins` | 3 | 0 (off) to 30 |
| `hotkeys` | `Ctrl+Shift+F9..F12` | One accelerator per action, see Global hotkeys |

#### HUD size

- One setting, `hudScale`, from **100% (minimum, the default size) to 200%**, stored in `settings.json` in the app-data directory and applied at startup. Changed from the main window's HUD section (slider plus −/+ buttons).
- The HUD content has a fixed base box (242 px wide + 4 px margin); the page applies CSS `zoom: hudScale` to it and the backend resizes the window to 250×105 logical px (250×123 with the chart row) × `hudScale`, so the layout is identical at any size.

#### Global hotkeys

Via `tauri-plugin-global-shortcut` (OS-level hotkey registration, no keyboard hooks). Defaults use `Ctrl+Shift+F9..F12` because they rarely collide with MapleStory keybinds. Every action can be rebound from the main window's **Shortcuts** section.

| Action | Default |
|---|---|
| Start / pause / resume | `Ctrl+Shift+F9` |
| Stop and save session | `Ctrl+Shift+F10` |
| Show / hide compact HUD | `Ctrl+Shift+F11` |
| Lock HUD (click-through) / unlock (draggable) | `Ctrl+Shift+F12` |

A registered hotkey is consumed by the OS, so the game never receives that combo.

Rebinding:
- Click a combo, then press the new keys; `Esc` cancels, and so does leaving the window. While recording, all global hotkeys are unregistered so a bound combo reaches the window, and they are registered again afterwards.
- The UI sends browser key codes (`Control+Alt+KeyE`); the backend parses and stores them normalized (`control+alt+KeyE`) in `settings.json`.
- A combo needs Ctrl, Alt or Cmd/Win, or a function key (F1–F24), so plain game keys are never taken.
- A combo already used by another action is rejected. A combo the OS refuses (taken by the system or another app) is rolled back to the previous binding with an error.
- Each action has a restore-default button, plus **Reset all**.
- At startup, any binding the OS refuses is shown as "Not active" in the Shortcuts section; the other bindings still work.

### 4.6 Persistence — CONFIRMED

- **SQLite** (`rusqlite`, bundled), single local file in the OS app-data directory.
- Schema designed so sessions can later be synced to a remote service.

## 5. Domain logic

### 5.1 EXP gain accounting

- Track `exp_current` over time, and accumulate `exp_gained` as the sum of positive deltas.
- **Level up:** the displayed EXP resets. Gain across a level-up is `(exp_to_level(prev_level) - prev_exp) + new_exp`. Uses the level read from the HUD plus the EXP table per level (public game data). The percentage gives a cross-check without the table.
- **Death / EXP loss: out of scope.** Character deaths are not considered. A negative delta without a level-up is treated as an invalid read.

### 5.2 Reading validation

A read is rejected when:
- It does not parse.
- The value decreases without a level-up.
- The jump is implausibly large for the elapsed time.
- The numeric value and the percentage disagree with each other.

### 5.3 Metrics

- Session average rate (total gained / active time).
- Rolling-window rate (e.g. last 1, 5, 15 min) to reflect the current spot.
- Time to next level at the chosen window's rate (falls back to the session rate).
- **Rate trend:** the 1-minute rate sampled every 20 s of active time, last 90 samples (30 active minutes).
- **Best session:** the highest EXP/h among saved sessions of at least 5 active minutes whose level range covers the current level, for the same character when the name is known. The live gap is `exp_gained / best_rate − active_secs`: the time the best pace would need to gain what this session gained, minus the time it actually took. Positive means ahead. Shown after 1 active minute. Looked up again when level or character changes and after a session is saved.

### 5.4 Session model — CONFIRMED (manual only)

- The user explicitly starts, pauses and stops a session. No auto-start.
- Elapsed time counts only while the session is running.
- **Idle auto-pause (optional, default 3 min):** when no EXP is gained for that long, the session pauses itself and gives back the idle stretch (active time and rate samples rewind to the last gain), so AFK time never dilutes the rates. While auto-paused the reader keeps reading; the first changed EXP reading resumes the session (a rejected read does not). A manual pause is never resumed automatically.
- Captures and OCR run only while a session is running or auto-paused (zero work otherwise).

## 6. Implementation status

### Project layout

| Path | Content |
|---|---|
| `crates/exp-core` | Pure Rust, no platform code: `ExpTracker` (gain accounting, validation, resync), `RollingRate`, `Session`, `ExpTable` |
| `crates/sysocr` | One-shot OS OCR (Apple Vision / Windows.Media.Ocr) for the character name |
| `crates/ocr` | Status bar OCR: label search, binarize, segment, template match, parse, self-learning; glyph assets and a real-capture test fixture |
| `crates/capture` | Game window lookup and capture (`xcap`), region crop, frame hash, OS screen permission |
| `src-tauri` | Tauri shell: `state` (shared state), `actions` (session/HUD operations), `hotkeys`, `storage` (SQLite), `detect` (window check + preview), `reader` (capture → OCR → reading, once per second while running), commands in `lib.rs` |
| `src/routes/+page.svelte` | Main window |
| `src/routes/hud/+page.svelte` | Compact HUD window |
| `src/lib` | Shared types, formatting, `Live` (event-driven backend state) |

### Data flow

1. While a session runs (or is auto-paused), the reader captures the game window once per second and produces `ExpReading { level, exp, percent }` (a debug-only dev feed can also push readings).
2. `Session::ingest` validates and accumulates it while running.
3. A 1 s ticker samples rates while running and pushes a `snapshot` event to both windows.
4. Stop saves a summary row to `sessions.db` in the OS app-data directory.

### Validation rules as implemented

- Percent outside 0–100: rejected, never resyncs.
- EXP decrease at the same level, level decrease, or exp/percent mismatch: rejected.
- After 3 consecutive rejections the tracker re-baselines on the latest reading without counting gain, so it never gets stuck.
- Required EXP per level comes from the EXP table when loaded, otherwise from `exp * 100 / percent` using the highest-percent reading at that level.

### Milestones

| # | Milestone | Status |
|---|---|---|
| M0 | Workspace, core logic with tests, windows, hotkeys, SQLite history, dev feed | Done |
| M1 | Window capture of the game window, ROI crop, change detection | Done |
| M2 | ROI selection: manual drag + auto-detection | Auto-detection done; manual pending |
| M3 | Glyph template OCR for EXP, %, level; OS OCR for name | Done |
| M4 | Settings, rate trend, best-session pace, idle auto-pause, rate window, HUD opacity, hover controls, level-up effect | Done |
| M5 | Rebindable hotkeys | Done |

## 7. Glossary

| Term | Meaning |
|---|---|
| HUD | Heads-Up Display: information drawn over the main view without stopping play. |
| Game status bar | The game's own HUD strip at the bottom of the window (LV, name, HP, MP, EXP). The data source. |
| Compact HUD | The tracker's small always-on-top window that shows the live EXP rate. |
| ROI | Region of interest: the rectangle of the game window that gets cropped and read. |

## 8. Conventions

- All documentation in English.
- Code comments in English, one line, no more.
- No co-author or AI attribution in commits, docs, or code.

## Open questions

None right now. New questions get added here as they come up.
