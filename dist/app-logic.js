//  STATE 
const T = [];
let cur = -1, playing = false, shuf = false, rep = 0, lkd = false, wvd = null, actx = null;
window.T = T;
Object.defineProperty(window, 'cur', { get: () => cur, set: (v) => cur = v });
let playEpoch = 0;
const aud = document.getElementById('aud');
aud.volume = 0.8;
let msPosTick = 0;
const PLAYLISTS_CACHE_KEY = 'liquify_playlists_v1';
const TRACKS_CACHE_KEY = 'liquify_tracks';
const OFFLINE_AUDIO_DB = 'liquify_offline_audio_v1';
const OFFLINE_AUDIO_STORE = 'tracks';
const LAST_TRACK_KEY = 'liquify_last_track_v1';
const OFFLINE_MODE_KEY = 'liquify_offline_mode';
let P = [];
let activePlaylistId = 'all';
let lastNonPlayerView = 'h';
let currentViewKey = 'h';
let trackPlaylistModalTrackId = null;
let searchProvider = 'local'; // 'local' | 'spotify' | 'soundcloud'
let searchCategory = 'tracks'; // 'tracks' | 'playlists' | 'artists'
let searchDebounce = null;
let offlineModeEnabled = false;
let _prefetchingNextTrack = false;
let _prefetchedStreamCache = new Map(); // trackId -> streamUrl
let _preloadedArtCache = new Map(); // trackId -> Image (keep ref to prevent GC)
const WAVEFORM_CACHE_KEY = 'liquify_waveform_cache_v1';
let playerCanvasRequestId = 0;
const spotifyCanvasCache = new Map();
let activeCanvasInfo = null;
const CANVAS_LOG_LIMIT = 80;
let canvasLogOverlayEnabled = false;
let canvasOverlayLines = [];
let waveformRequestId = 0;
const waveformCache = new Map();

async function loadAppStateValue(key) {
  const inv = getTauriInvoke();
  if (typeof inv !== 'function') return localStorage.getItem(key);
  try {
    const val = await inv('load_app_state', { key });
    if (typeof val === 'string') {
      localStorage.setItem(key, val);
      return val;
    }
  } catch (_) { }
  return localStorage.getItem(key);
}

function saveAppStateValue(key, value) {
  try { localStorage.setItem(key, value); } catch (_) { }
  const inv = getTauriInvoke();
  if (typeof inv === 'function') {
    inv('save_app_state', { key, value }).catch(() => { });
  }
}

// Logging system for UI
let debugLogs = [];
function addDebugLog(msg) {
  debugLogs.push(msg);
  if (debugLogs.length > 50) debugLogs.shift(); // Limit to 50 recent logs
}
function showDebugLogs(title = 'Ошибка') {
  const html = `<div style="color:red;padding:20px;text-align:left;font-family:monospace;font-size:14px;line-height:1.6;max-height:400px;overflow-y:auto;background:rgba(255,0,0,0.05);border-radius:8px">
    <strong style="color:#ff4444;font-size:16px">${title}:</strong><br/>
    ${debugLogs.map(log => `<div>${escapeHtml(log)}</div>`).join('')}
  </div>`;
  setLyricsStatus(title);
  document.getElementById('lx-ov-list').innerHTML = html;
}
function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function getCanvasLogTimestamp() {
  const d = new Date();
  const hh = String(d.getHours()).padStart(2, '0');
  const mm = String(d.getMinutes()).padStart(2, '0');
  const ss = String(d.getSeconds()).padStart(2, '0');
  const ms = String(d.getMilliseconds()).padStart(3, '0');
  return `${hh}:${mm}:${ss}.${ms}`;
}

function renderCanvasLogOverlay() {
  const ov = document.getElementById('canvas-log-ov');
  const list = document.getElementById('canvas-log-list');
  const btn = document.getElementById('canvas-log-toggle-btn');
  if (ov) ov.classList.toggle('show', !!canvasLogOverlayEnabled);
  if (btn) btn.textContent = canvasLogOverlayEnabled ? 'Скрыть логи' : 'Показать логи';
  if (!list) return;
  if (!canvasLogOverlayEnabled) {
    list.innerHTML = '';
    return;
  }
  if (!canvasOverlayLines.length) {
    list.innerHTML = '<div class="canvas-log-line">CANVAS LOGS waiting...</div>';
    return;
  }
  list.innerHTML = canvasOverlayLines.map((line) => `<div class="canvas-log-line">${escapeHtml(line)}</div>`).join('');
}

function pushCanvasOverlayLog(level, payload) {
  if (!canvasLogOverlayEnabled) return;
  const line = `${getCanvasLogTimestamp()} ${String(level || 'INFO').toUpperCase()} ${String(payload || '')}`;
  canvasOverlayLines.push(line);
  if (canvasOverlayLines.length > CANVAS_LOG_LIMIT) {
    canvasOverlayLines = canvasOverlayLines.slice(-CANVAS_LOG_LIMIT);
  }
  renderCanvasLogOverlay();
}

function toggleCanvasLogOverlay() {
  canvasLogOverlayEnabled = !canvasLogOverlayEnabled;
  if (canvasLogOverlayEnabled) {
    canvasOverlayLines = [];
    pushCanvasOverlayLog('info', 'CANVAS overlay enabled');
  } else {
    renderCanvasLogOverlay();
  }
}

async function copyCanvasLogs(event) {
  if (event) {
    event.preventDefault();
    event.stopPropagation();
  }
  const text = canvasOverlayLines.join('\n');
  if (!text) return;
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
    } else {
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.position = 'fixed';
      ta.style.opacity = '0';
      document.body.appendChild(ta);
      ta.focus();
      ta.select();
      document.execCommand('copy');
      ta.remove();
    }
    scToast('Canvas логи', 'Логи скопированы', false);
    setTimeout(scToastHide, 1200);
  } catch (_) {
    scToast('Canvas логи', 'Не удалось скопировать', false);
    setTimeout(scToastHide, 1500);
  }
}
window.toggleCanvasLogOverlay = toggleCanvasLogOverlay;
window.copyCanvasLogs = copyCanvasLogs;

// Background animation settings
const BG_ANIM_KEY = 'liquify_bg_anim_enabled';
const BG_BLUR_KEY = 'liquify_bg_blur_val';
const UI_FPS_ENABLED_KEY = 'liquify_ui_fps_enabled';
const ASSETS_DB_NAME = 'liquify_assets_v1';
const WALLPAPER_KEY = 'custom_wallpaper';
const WALLPAPER_TYPE_KEY = 'custom_wallpaper_type'; // 'image' or 'video'
const UI_FPS_HUD_ID = 'ui-fps-hud';
const UI_FPS_STYLE_ID = 'ui-fps-style';

const uiFpsState = {
  rafId: 0,
  lastTs: 0,
  bucketStart: 0,
  bucketFrames: 0,
  currentFps: 0,
  peakFps: 0,
  frameMs: 0,
  refreshHz: 0,
  refreshSamples: [],
  refreshSampleLimit: 90,
  hudEl: null
};

function ensureUiFpsHud() {
  if (!document.body) return null;

  let style = document.getElementById(UI_FPS_STYLE_ID);
  if (!style) {
    style = document.createElement('style');
    style.id = UI_FPS_STYLE_ID;
    style.textContent = `
      #${UI_FPS_HUD_ID} {
        position: fixed;
        top: max(10px, env(safe-area-inset-top, 0px) + 6px);
        right: max(10px, env(safe-area-inset-right, 0px) + 6px);
        z-index: 99999;
        pointer-events: none;
        padding: 7px 10px;
        border-radius: 12px;
        border: 1px solid rgba(255,255,255,.14);
        background: rgba(5, 7, 12, .68);
        color: #f3f7ff;
        font: 700 11px/1.25 "Plus Jakarta Sans", system-ui, sans-serif;
        letter-spacing: .04em;
        backdrop-filter: blur(14px) saturate(1.35);
        -webkit-backdrop-filter: blur(14px) saturate(1.35);
        box-shadow: 0 10px 35px rgba(0,0,0,.28);
        text-align: right;
        min-width: 126px;
      }
      #${UI_FPS_HUD_ID} .ui-fps-label {
        display: block;
        color: rgba(255,255,255,.58);
        font-size: 9px;
        letter-spacing: .12em;
        text-transform: uppercase;
        margin-bottom: 2px;
      }
      #${UI_FPS_HUD_ID} .ui-fps-value {
        display: block;
        font-size: 16px;
        font-weight: 800;
        letter-spacing: .02em;
      }
      #${UI_FPS_HUD_ID} .ui-fps-meta {
        display: block;
        margin-top: 2px;
        color: rgba(255,255,255,.72);
        font-size: 10px;
        font-weight: 600;
      }
    `;
    document.head.appendChild(style);
  }

  let hud = document.getElementById(UI_FPS_HUD_ID);
  if (!hud) {
    hud = document.createElement('div');
    hud.id = UI_FPS_HUD_ID;
    hud.innerHTML = `
      <span class="ui-fps-label">UI FPS</span>
      <span class="ui-fps-value">0.0</span>
      <span class="ui-fps-meta">0.0 ms | peak 0.0</span>
    `;
    document.body.appendChild(hud);
  }

  hud.style.display = localStorage.getItem(UI_FPS_ENABLED_KEY) === 'true' ? '' : 'none';

  uiFpsState.hudEl = hud;
  return hud;
}

function estimateRefreshHzFromSamples() {
  if (uiFpsState.refreshSamples.length < 12) return 0;
  const sorted = [...uiFpsState.refreshSamples].sort((a, b) => a - b);
  const median = sorted[Math.floor(sorted.length / 2)];
  if (!median || median <= 0) return 0;
  return 1000 / median;
}

function updateUiFpsHud() {
  const hud = ensureUiFpsHud();
  if (!hud) return;
  const valueEl = hud.querySelector('.ui-fps-value');
  const metaEl = hud.querySelector('.ui-fps-meta');
  if (valueEl) valueEl.textContent = uiFpsState.currentFps.toFixed(1);
  if (metaEl) {
    const hzText = uiFpsState.refreshHz > 0 ? ` | hz ${uiFpsState.refreshHz.toFixed(1)}` : '';
    metaEl.textContent = `${uiFpsState.frameMs.toFixed(1)} ms | peak ${uiFpsState.peakFps.toFixed(1)}${hzText}`;
  }
}

function uiFpsTick(ts) {
  if (document.hidden) {
    uiFpsState.lastTs = ts;
    uiFpsState.bucketStart = ts;
    uiFpsState.bucketFrames = 0;
    uiFpsState.rafId = requestAnimationFrame(uiFpsTick);
    return;
  }

  if (!uiFpsState.lastTs) {
    uiFpsState.lastTs = ts;
    uiFpsState.bucketStart = ts;
    uiFpsState.rafId = requestAnimationFrame(uiFpsTick);
    return;
  }

  const delta = ts - uiFpsState.lastTs;
  uiFpsState.lastTs = ts;
  if (delta > 0) {
    const instantFps = 1000 / delta;
    uiFpsState.frameMs = delta;
    uiFpsState.peakFps = Math.max(uiFpsState.peakFps, instantFps);
    if (uiFpsState.refreshSamples.length < uiFpsState.refreshSampleLimit) {
      uiFpsState.refreshSamples.push(delta);
      uiFpsState.refreshHz = estimateRefreshHzFromSamples();
    }
  }

  uiFpsState.bucketFrames += 1;
  const bucketElapsed = ts - uiFpsState.bucketStart;
  if (bucketElapsed >= 250) {
    uiFpsState.currentFps = (uiFpsState.bucketFrames * 1000) / bucketElapsed;
    uiFpsState.bucketFrames = 0;
    uiFpsState.bucketStart = ts;
    updateUiFpsHud();
  }

  uiFpsState.rafId = requestAnimationFrame(uiFpsTick);
}

function startUiFpsMonitor() {
  ensureUiFpsHud();
  if (uiFpsState.rafId) return;
  uiFpsState.rafId = requestAnimationFrame(uiFpsTick);
}

function stopUiFpsMonitor() {
  if (uiFpsState.rafId) {
    cancelAnimationFrame(uiFpsState.rafId);
    uiFpsState.rafId = 0;
  }
}

function setUiFpsEnabled(enabled) {
  const on = !!enabled;
  localStorage.setItem(UI_FPS_ENABLED_KEY, on ? 'true' : 'false');
  const hud = ensureUiFpsHud();
  if (hud) hud.style.display = on ? '' : 'none';
  const toggle = document.getElementById('ui-fps-toggle');
  if (toggle) toggle.checked = on;
  if (on) startUiFpsMonitor();
  else stopUiFpsMonitor();
}

function initUiFpsSetting() {
  const enabled = localStorage.getItem(UI_FPS_ENABLED_KEY) === 'true';
  const toggle = document.getElementById('ui-fps-toggle');
  if (toggle) toggle.checked = enabled;
  setUiFpsEnabled(enabled);
}

async function openAssetsDB() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(ASSETS_DB_NAME, 1);
    request.onupgradeneeded = (e) => {
      const db = e.target.result;
      if (!db.objectStoreNames.contains('assets')) {
        db.createObjectStore('assets');
      }
    };
    request.onsuccess = (e) => resolve(e.target.result);
    request.onerror = (e) => reject(e.target.error);
  });
}

async function saveAsset(key, value) {
  const db = await openAssetsDB();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(['assets'], 'readwrite');
    const store = transaction.objectStore('assets');
    const request = store.put(value, key);
    request.onsuccess = () => resolve();
    request.onerror = (e) => reject(e.target.error);
  });
}

async function getAsset(key) {
  const db = await openAssetsDB();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(['assets'], 'readonly');
    const store = transaction.objectStore('assets');
    const request = store.get(key);
    request.onsuccess = (e) => resolve(e.target.result);
    request.onerror = (e) => reject(e.target.error);
  });
}

async function deleteAsset(key) {
  const db = await openAssetsDB();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(['assets'], 'readwrite');
    const store = transaction.objectStore('assets');
    const request = store.delete(key);
    request.onsuccess = () => resolve();
    request.onerror = (e) => reject(e.target.error);
  });
}

async function handleWallpaperSelection(input, type = 'image') {
  const file = input.files[0];
  if (!file) return;

  if (type === 'video') {
    // Save video as blob in IndexedDB for persistence
    const arrayBuffer = await file.arrayBuffer();
    const videoBlob = new Blob([arrayBuffer], { type: file.type });
    const videoUrl = URL.createObjectURL(videoBlob);

    // Save blob to IndexedDB
    await saveAsset(WALLPAPER_KEY, videoBlob);
    await saveAsset(WALLPAPER_TYPE_KEY, 'video');

    applyWallpaper(videoUrl, 'video');
    document.getElementById('wall-reset').style.display = 'block';
  } else {
    // For image, use data URL
    const reader = new FileReader();
    reader.onload = async (e) => {
      const dataUrl = e.target.result;
      applyWallpaper(dataUrl, 'image');
      await saveAsset(WALLPAPER_KEY, dataUrl);
      await saveAsset(WALLPAPER_TYPE_KEY, 'image');
      document.getElementById('wall-reset').style.display = 'block';
    }
    reader.readAsDataURL(file);
  }
  input.value = '';
}

function applyWallpaper(url, type = 'image') {
  const bgw = document.getElementById('bgw');
  const bgVideo = document.getElementById('bg-video');

  if (!bgw || !bgVideo) return;

  // Revoke previous object URL if exists
  if (currentWallpaperUrl && currentWallpaperUrl.startsWith('blob:')) {
    URL.revokeObjectURL(currentWallpaperUrl);
  }
  currentWallpaperUrl = url && url.startsWith('blob:') ? url : null;

  // Reset both
  bgw.style.backgroundImage = '';
  bgw.classList.remove('on');
  bgVideo.style.display = 'none';
  bgVideo.classList.remove('on');
  bgVideo.pause();
  bgVideo.removeAttribute('src');

  if (!url) return;

  if (type === 'video') {
    // Apply video background
    bgVideo.src = url;
    bgVideo.style.display = 'block';
    bgVideo.classList.add('on');
    // Ensure video plays
    bgVideo.play().catch(e => console.warn('[wallpaper] video play failed:', e));
  } else {
    // Apply image background
    bgw.style.backgroundImage = `url(${url})`;
    bgw.classList.add('on');
  }
}

let currentWallpaperUrl = null;

async function resetWallpaper() {
  // Revoke object URL if exists
  if (currentWallpaperUrl && currentWallpaperUrl.startsWith('blob:')) {
    URL.revokeObjectURL(currentWallpaperUrl);
    currentWallpaperUrl = null;
  }

  await deleteAsset(WALLPAPER_KEY);
  await deleteAsset(WALLPAPER_TYPE_KEY);
  applyWallpaper(null);
  document.getElementById('wall-reset').style.display = 'none';
}

async function initCustomWallpaper() {
  try {
    const wallpaperData = await getAsset(WALLPAPER_KEY);
    const wallpaperType = await getAsset(WALLPAPER_TYPE_KEY) || 'image';

    if (wallpaperData) {
      let url;
      if (wallpaperType === 'video') {
        // For video, create object URL from blob
        if (wallpaperData instanceof Blob) {
          url = URL.createObjectURL(wallpaperData);
        } else {
          // Handle case where it might be stored as data URL
          url = wallpaperData;
        }
      } else {
        url = wallpaperData;
      }
      applyWallpaper(url, wallpaperType);
      const resetBtn = document.getElementById('wall-reset');
      if (resetBtn) resetBtn.style.display = 'block';
    }
  } catch (e) {
    console.warn('[wallpaper] init failed', e);
  }
}

function updateBgBlur(val) {
  const px = val + 'px';
  document.documentElement.style.setProperty('--bg-blur', px);
  localStorage.setItem(BG_BLUR_KEY, val);
  const info = document.getElementById('bg-blur-val');
  if (info) info.textContent = px;

  // Apply blur to video element if it's active
  const bgVideo = document.getElementById('bg-video');
  if (bgVideo && bgVideo.classList.contains('on')) {
    bgVideo.style.filter = `blur(${px})`;
  }
}

function initBgBlur() {
  const val = localStorage.getItem(BG_BLUR_KEY) || '70';
  updateBgBlur(val);
  const range = document.getElementById('bg-blur-range');
  if (range) range.value = val;
}

function toggleBgAnimation() {
  const checkbox = document.getElementById('bg-anim-toggle');
  const bgmEl = document.getElementById('bgm');
  if (!bgmEl || !checkbox) return;
  const enabled = checkbox.checked;
  localStorage.setItem(BG_ANIM_KEY, JSON.stringify(enabled));
  if (enabled) {
    bgmEl.classList.add('animated');
  } else {
    bgmEl.classList.remove('animated');
  }
}

function initBgAnimation() {
  const bgmEl = document.getElementById('bgm');
  const checkbox = document.getElementById('bg-anim-toggle');
  if (!bgmEl || !checkbox) return;
  const enabled = JSON.parse(localStorage.getItem(BG_ANIM_KEY) || 'false');
  checkbox.checked = enabled;
  if (enabled) {
    bgmEl.classList.add('animated');
  }
}

function setupPlayerCanvasLayer() {
  const video = document.getElementById('pcv-video');
  if (!video || video.dataset.bound) return;
  video.dataset.bound = '1';
  video.muted = true;
  video.loop = true;
  video.playsInline = true;
  video.setAttribute('webkit-playsinline', 'true');
  ['loadstart', 'loadedmetadata', 'canplay', 'playing', 'stalled', 'suspend', 'waiting', 'ended'].forEach((eventName) => {
    video.addEventListener(eventName, () => {
      logRustConsole('canvas-video', `${eventName} src=${(video.currentSrc || video.src || '').slice(0, 160)} readyState=${video.readyState} networkState=${video.networkState}`);
      canvasConsole(`video event ${eventName}`, {
        src: (video.currentSrc || video.src || '').slice(0, 200),
        readyState: video.readyState,
        networkState: video.networkState,
        currentTime: Number.isFinite(video.currentTime) ? Number(video.currentTime.toFixed(3)) : null
      });
      if (eventName === 'loadedmetadata' || eventName === 'canplay' || eventName === 'playing') {
        promotePlayerCanvasReady(eventName);
      }
    });
  });
  video.addEventListener('error', () => {
    canvasConsoleError('video element error', {
      code: video.error?.code || null,
      message: video.error?.message || null,
      src: (video.currentSrc || video.src || '').slice(0, 200),
      trackId: activeCanvasInfo?.trackId || null
    });
    const err = video.error;
    logRustConsole('canvas-video', `error code=${err?.code || 'none'} message=${err?.message || 'n/a'} src=${(video.currentSrc || video.src || '').slice(0, 160)}`);
    if (activeCanvasInfo?.proxiedCanvasUrl && video.dataset.canvasFallbackTried !== '1') {
      video.dataset.canvasFallbackTried = '1';
      const fallbackSrc = activeCanvasInfo.proxiedCanvasUrl;
      logRustConsole('canvas-video', `trying proxy fallback src=${fallbackSrc.slice(0, 160)}`);
      canvasConsoleWarn('trying proxied canvas fallback', {
        trackId: activeCanvasInfo?.trackId || null,
        fallbackSrc: fallbackSrc.slice(0, 200)
      });
      video.src = fallbackSrc;
      const retry = video.play();
      if (retry && typeof retry.catch === 'function') {
        retry.catch((playErr) => {
          canvasConsoleError('proxy fallback play failed', {
            trackId: activeCanvasInfo?.trackId || null,
            error: playErr?.message || String(playErr)
          });
          logRustConsole('canvas-video', `proxy fallback play failed ${playErr?.message || String(playErr)}`);
        });
      }
      return;
    }
    clearPlayerCanvasLayer();
  });
}

function promotePlayerCanvasReady(reason) {
  const vp = document.getElementById('vp');
  const video = document.getElementById('pcv-video');
  if (!vp || !video || !activeCanvasInfo?.trackId) return;
  const pendingTrackId = video.dataset.canvasPendingTrackId || '';
  const currentTrackId = T[cur]?.spotifyId || '';
  if (!pendingTrackId || pendingTrackId !== activeCanvasInfo.trackId) return;
  if (!currentTrackId || currentTrackId !== activeCanvasInfo.trackId) return;
  if (vp.classList.contains('has-canvas')) return;
  vp.classList.add('has-canvas');
  canvasConsole('canvas promoted to visible', {
    trackId: activeCanvasInfo.trackId,
    reason,
    src: (video.currentSrc || video.src || '').slice(0, 200)
  });
  showCanvasToast('Canvas Spotify', 'Канвас отображен', false, 2200);
}

function initAll() {
  loadWaveformCache();
  setupRustConsoleBridge();
  renderCanvasLogOverlay();
  initUiFpsSetting();
  setupLibraryChrome();
  setupSearchCategoryChips();
  setupPlayerSheetGestures();
  setupPlayerArtworkSwipeGestures();
  setupPlayerCanvasLayer();
  initBgAnimation();
  initBgBlur();
  initCustomWallpaper();
}

// Cleanup on page unload
window.addEventListener('beforeunload', () => {
  if (currentWallpaperUrl && currentWallpaperUrl.startsWith('blob:')) {
    URL.revokeObjectURL(currentWallpaperUrl);
    currentWallpaperUrl = null;
  }
  const bgVideo = document.getElementById('bg-video');
  if (bgVideo) {
    bgVideo.pause();
    bgVideo.removeAttribute('src');
  }
});

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initAll);
} else {
  initAll();
}


function hashStr(s) {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); }
  return (h >>> 0).toString(36);
}

function getWaveformKey(t) {
  if (!t) return '';
  ensureTrackId(t);
  return String(
    t.id ||
    t.spotifyId ||
    t.scTrackId ||
    t.path ||
    t.streamUrl ||
    t.url ||
    `${t.title || ''}|${t.artist || ''}|${t.album || ''}`
  );
}

function normalizeWaveform(peaks, bars) {
  const count = Math.max(8, Math.min(180, Number(bars) || 90));
  if (!Array.isArray(peaks) || !peaks.length) return null;
  const src = peaks.map(v => Math.max(0, Number(v) || 0));
  if (!src.length) return null;
  const out = [];
  for (let i = 0; i < count; i++) {
    const start = Math.floor((i / count) * src.length);
    const end = Math.max(start + 1, Math.floor(((i + 1) / count) * src.length));
    let sum = 0;
    let n = 0;
    for (let j = start; j < end && j < src.length; j++) {
      sum += src[j];
      n++;
    }
    out.push(n ? sum / n : 0);
  }
  const mx = out.reduce((m, v) => Math.max(m, v), 0) || 1;
  return out.map(v => {
    const normalized = v / mx;
    return Math.min(1, Math.max(0.08, normalized));
  });
}

function buildWaveformPlaceholder(t, bars) {
  const count = Math.max(8, Math.min(180, Number(bars) || 90));
  const seed = hashStr(getWaveformKey(t) || `${t?.title || ''}|${t?.artist || ''}`);
  const seedNum = parseInt(seed, 36) || 1;
  const out = [];
  for (let i = 0; i < count; i++) {
    const a = Math.sin((i + 1) * 0.33 + (seedNum % 17) * 0.11);
    const b = Math.cos((i + 1) * 0.21 + (seedNum % 23) * 0.07);
    const c = Math.sin((i + 1) * 0.07 + (seedNum % 31) * 0.03);
    const mix = (a * 0.45 + b * 0.35 + c * 0.2 + 1) / 2;
    out.push(0.18 + mix * 0.62);
  }
  return out;
}

function loadWaveformCache() {
  try {
    const raw = localStorage.getItem(WAVEFORM_CACHE_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object') return;
    Object.entries(parsed).forEach(([key, peaks]) => {
      const normalized = normalizeWaveform(peaks, 90);
      if (key && normalized) waveformCache.set(key, normalized);
    });
  } catch (_) { }
}

function persistWaveformCache() {
  try {
    const entries = Array.from(waveformCache.entries()).slice(-80);
    const payload = {};
    entries.forEach(([key, peaks]) => {
      if (key && Array.isArray(peaks) && peaks.length) payload[key] = peaks;
    });
    localStorage.setItem(WAVEFORM_CACHE_KEY, JSON.stringify(payload));
  } catch (_) { }
}

function setWaveformData(t, peaks, persist) {
  const key = getWaveformKey(t);
  const normalized = normalizeWaveform(peaks, 90);
  if (!key || !normalized) return false;
  waveformCache.set(key, normalized);
  wvd = normalized;
  if (persist) persistWaveformCache();
  return true;
}

function ensureTrackId(track) {
  if (track.id) return track.id;
  const stable = track.streamUrl || track.path || track.url || `${track.title}|${track.artist}|${track.album}`;
  track.id = 't_' + hashStr(String(stable));
  return track.id;
}

function scTrackIdFromAny(item) {
  if (!item) return '';
  if (item.id != null && String(item.id).trim()) return String(item.id).trim();
  const urn = item.urn ? String(item.urn) : '';
  const m = urn.match(/soundcloud:tracks:(.+)$/);
  return m ? String(m[1]).trim() : '';
}

function trackById(id) {
  return T.find(x => x.id === id) || null;
}

function basePlaylists() {
  return [
    { id: 'all', name: 'Все треки', description: '', trackIds: [], locked: true },
    { id: 'favorites', name: 'Избранное', description: 'Ваши любимые треки', trackIds: [], locked: true },
    { id: 'recent', name: 'Недавно играло', description: 'Последние прослушанные треки', trackIds: [], locked: true }
  ];
}

function savePlaylistsCache() {
  try {
    // Fix: Allow saving 'favorites' even if it's locked. Only exclude 'all' as it is purely dynamic.
    const payload = P.filter(x => x.id !== 'all').map(x => ({
      id: x.id, name: x.name, description: x.description || '', trackIds: Array.from(new Set(x.trackIds || [])),
      source: x.source || 'local', sourceUrl: x.sourceUrl || '', spotifyId: x.spotifyId || ''
    }));
    saveAppStateValue(PLAYLISTS_CACHE_KEY, JSON.stringify(payload));
  } catch (e) { }
}

function isTrackReferencedBySavedPlaylist(trackId) {
  if (!trackId) return false;
  return P.some(pl => pl && pl.id !== 'all' && !pl._preview && Array.isArray(pl.trackIds) && pl.trackIds.includes(trackId));
}

function clearPreviewFlagsForPlaylistTracks(pl) {
  if (!pl || !Array.isArray(pl.trackIds)) return;
  pl.trackIds.forEach(id => {
    const track = trackById(id);
    if (!track) return;
    delete track._previewTransient;
  });
}

async function loadPlaylistsCache() {
  P = basePlaylists();
  try {
    const raw = await loadAppStateValue(PLAYLISTS_CACHE_KEY);
    if (!raw) return;
    const arr = JSON.parse(raw);
    if (!Array.isArray(arr)) return;
    arr.forEach(pl => {
      if (!pl || !pl.id || !pl.name) return;

      // Fix: Merge with existing base playlists (like favorites) instead of duplicating
      const existing = P.find(x => x.id === String(pl.id));
      if (existing) {
        existing.trackIds = Array.isArray(pl.trackIds) ? pl.trackIds.map(String) : [];
        if (pl.description) existing.description = String(pl.description);
      } else {
        P.push({
          id: String(pl.id),
          name: String(pl.name),
          description: String(pl.description || ''),
          trackIds: Array.isArray(pl.trackIds) ? pl.trackIds.map(String) : [],
          source: String(pl.source || 'local'),
          sourceUrl: String(pl.sourceUrl || ''),
          spotifyId: String(pl.spotifyId || '')
        });
      }
    });
  } catch (e) { }
}

function getActivePlaylist() {
  return P.find(x => x.id === activePlaylistId) || P[0];
}

function getVisibleTrackEntries() {
  T.forEach(ensureTrackId);
  const fav = P.find(x => x.id === 'favorites');
  const favIds = new Set(fav ? fav.trackIds : []);
  T.forEach(t => { t.liked = favIds.has(t.id); });

  // Always use current index from T (after any sort/mutation)
  if (activePlaylistId === 'all') return T.map((t, idx) => ({ track: t, idx }));
  const pl = getActivePlaylist();
  if (!pl) return T.map((t, idx) => ({ track: t, idx }));
  const set = new Set(pl.trackIds || []);
  const out = [];
  T.forEach((t, idx) => { if (set.has(t.id)) out.push({ track: t, idx }); });
  return out;
}

function getPlaybackEntries() {
  const entries = getVisibleTrackEntries().filter(entry => entry && entry.track);
  if (offlineModeEnabled) {
    return entries.filter(({ track: t }) => t._offlineCached || (t.file && t.url && t.url.startsWith('blob:')));
  }
  return entries;
}

function getPlaybackPosition(entries, trackIndex) {
  return entries.findIndex(entry => entry.idx === trackIndex);
}

function getNextPlaybackIndex() {
  const entries = getPlaybackEntries();
  if (!entries.length) return -1;
  if (rep === 2 && cur >= 0) return cur;
  if (shuf) {
    if (entries.length === 1) return entries[0].idx;
    const pool = entries.filter(entry => entry.idx !== cur);
    const pick = pool.length ? pool[Math.floor(Math.random() * pool.length)] : entries[0];
    return pick ? pick.idx : -1;
  }
  const pos = getPlaybackPosition(entries, cur);
  if (pos < 0) return entries[0].idx;
  return entries[(pos + 1) % entries.length].idx;
}

function getPrevPlaybackIndex() {
  const entries = getPlaybackEntries();
  if (!entries.length) return -1;
  const pos = getPlaybackPosition(entries, cur);
  if (pos < 0) return entries[0].idx;
  return entries[(pos - 1 + entries.length) % entries.length].idx;
}

function deletePlaylist(plId) {
  const idx = P.findIndex(x => x.id === plId);
  if (idx < 0) return;
  if (P[idx].locked) return;
  P.splice(idx, 1);
  if (activePlaylistId === plId) activePlaylistId = 'all';
  savePlaylistsCache();
  renderPlaylists();
  rl();
}

function pushRecentTrack(t) {
  if (!t) return;
  ensureTrackId(t);
  const recent = P.find(x => x.id === 'recent');
  if (!recent) return;
  recent.trackIds = (recent.trackIds || []).filter(id => id !== t.id);
  recent.trackIds.unshift(t.id);
  if (recent.trackIds.length > 50) recent.trackIds = recent.trackIds.slice(0, 50);
  savePlaylistsCache();
  renderPlaylists();
}

function openHomePlaylist(plId) {
  const pl = P.find(x => x.id === plId);
  if (!pl) return;
  activePlaylistId = plId;
  openPlaylistView(plId);
}
window.openHomePlaylist = openHomePlaylist;

// Playlist glass logic removed; now handled by global liquifyEngine in index.html

function setupLibraryChrome() {
  const view = document.getElementById('vl');
  const header = view ? view.querySelector('.lhd') : null;
  if (header && !header.dataset.libraryChrome) {
    header.dataset.libraryChrome = '1';
    header.classList.add('lib-head');
    header.innerHTML = `
      <div class="lib-head-main">
        <div class="lib-avatar">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="9"></circle>
            <path d="M12 7v5l3 2"></path>
          </svg>
        </div>
        <div class="lib-title">Моя медиатека</div>
      </div>
      <div class="lib-actions">
        <div class="addbtn" id="addbtn" title="Нажмите — добавить файлы / Зажмите — SoundCloud из буфера">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
        </div>
      </div>
    `;
  }

  if (view && !view.querySelector('.lib-chips')) {
    const chips = document.createElement('div');
    chips.className = 'lib-chips';
    chips.innerHTML = `
      <button class="lib-chip active" id="lib-chip-playlists" onclick="setLibraryCategory('playlists')">Плейлисты</button>
      <button class="lib-chip" id="lib-chip-artists" onclick="setLibraryCategory('artists')">Исполнители</button>
      <button class="lib-chip" id="lib-chip-albums" onclick="setLibraryCategory('albums')">Альбомы</button>
    `;
    header?.insertAdjacentElement('afterend', chips);
  }

  const sortBtn = document.getElementById('sortbtn');
  if (sortBtn) sortBtn.style.display = 'none';
  const addBtn = document.getElementById('addbtn');
  const fi = document.getElementById('fi');
  if (addBtn && fi && !addBtn.dataset.libraryBound) {
    addBtn.dataset.libraryBound = '1';
    addBtn.addEventListener('click', () => fi.click());
  }
}

function setLibraryCategory(kind) {
  ['playlists', 'artists', 'albums'].forEach((name) => {
    const chip = document.getElementById(`lib-chip-${name}`);
    if (!chip) return;
    chip.classList.toggle('active', name === 'playlists');
  });
  if (kind !== 'playlists') {
    scToast('Пока недоступно', 'Сейчас в плеере работают только плейлисты', false);
    setTimeout(scToastHide, 2200);
  }
}

function setPlayerOverlayState(open) {
  document.body.classList.toggle('player-overlay-open', !!open);
  throttleBackgroundViews(open);
}

function throttleBackgroundViews(throttle) {
  document.querySelectorAll('.view:not(#vp)').forEach(v => {
    if (throttle) {
      v.dataset.throttled = '1';
    } else {
      delete v.dataset.throttled;
    }
  });
}

function detectActiveViewKey() {
  const activeView = document.querySelector('.view.active:not(#vp)');
  if (!activeView) return currentViewKey || lastNonPlayerView || 'h';
  const map = { vh: 'h', vl: 'l', vs: 's', vsett: 'sett' };
  return map[activeView.id] || currentViewKey || lastNonPlayerView || 'h';
}

function syncNavSelection(viewKey) {
  const nb = { h: 'nbh', l: 'nbl', s: 'nbs', sett: 'nbsett', vpl: 'nbl', vartist: 'nbl' };
  document.querySelectorAll('.nb').forEach(b => b.classList.remove('on'));
  const id = nb[viewKey];
  if (id) document.getElementById(id)?.classList.add('on');
}

function preparePlayerOpenAnimation() {
  const vp = document.getElementById('vp');
  if (!vp) return;
  // Use fixed 100vh instead of getBoundingClientRect() to avoid layout thrashing
  vp.style.setProperty('--player-open-y', `100vh`);
  vp.style.setProperty('--player-open-radius', `28px`);
}

function openPlayerSheet() {
  const vp = document.getElementById('vp');
  const mini = document.getElementById('mini');
  const backTo = detectActiveViewKey();
  currentViewKey = backTo;
  lastNonPlayerView = backTo;
  if (vp) {
    vp.classList.remove('player-dragging');
    vp.style.transform = '';
    preparePlayerOpenAnimation();
    
    requestAnimationFrame(() => {
      vp.classList.add('active');
      vp.style.setProperty('--player-open-y', '0px');
      vp.style.setProperty('--player-open-radius', '0px');
    });
  }
  setPlayerOverlayState(true);
  mini?.classList.remove('show');
  syncNavSelection(backTo);
}

function closePlayerSheet() {
  const backTo = detectActiveViewKey();
  const vp = document.getElementById('vp');
  if (vp) {
    vp.classList.remove('player-dragging');
    vp.style.transform = '';
    vp.classList.remove('active');
    vp.style.setProperty('--player-open-y', '100vh');
    vp.style.setProperty('--player-open-radius', '28px');
  }
  setPlayerOverlayState(false);
  currentViewKey = backTo;
  lastNonPlayerView = backTo;
  syncNavSelection(backTo);
  if (cur >= 0) document.getElementById('mini')?.classList.add('show');
}

function setupPlayerSheetGestures() {
  const vp = document.getElementById('vp');
  if (!vp || vp.dataset.sheetBound) return;
  vp.dataset.sheetBound = '1';

  let startY = 0;
  let startX = 0;
  let dragging = false;
  let offsetY = 0;
  let isScrollingInner = false;

  const reset = () => {
    dragging = false;
    offsetY = 0;
    isScrollingInner = false;
    vp.classList.remove('player-dragging');
    vp.style.transform = '';
  };

  vp.addEventListener('touchstart', (e) => {
    if (!vp.classList.contains('active') || !e.touches?.length) return;
    
    // Smart scroll detection
    const target = e.target;
    const scrollable = target.closest('#lx-sec-b, .pinf');
    if (scrollable && scrollable.scrollTop > 0) {
      isScrollingInner = true;
      return;
    }
    isScrollingInner = false;
    
    startY = e.touches[0].clientY;
    startX = e.touches[0].clientX;
    dragging = true;
    offsetY = 0;
    vp.classList.add('player-dragging');
    vp.style.transition = 'none';
  }, { passive: true });

  vp.addEventListener('touchmove', (e) => {
    if (!dragging || !e.touches?.length || isScrollingInner) return;
    const dy = e.touches[0].clientY - startY;
    const dx = e.touches[0].clientX - startX;
    
    if (Math.abs(dx) > Math.abs(dy) && Math.abs(dx) > 15) {
      // Horizontal swipe detected
      if (offsetY < 10) {
        reset();
        vp.style.transition = '';
      }
      return;
    }
    
    if (dy > 0) {
      offsetY = dy;
      vp.style.transform = `translateY(${offsetY}px)`;
    }
  }, { passive: true });

  const finish = () => {
    if (!dragging) return;
    const shouldClose = offsetY > 100;
    vp.style.transition = '';
    
    if (shouldClose) {
      closePlayerSheet();
      // Keep position until CSS transition takes over
      vp.style.transform = `translateY(${window.innerHeight}px)`;
    } else {
      vp.style.transform = '';
    }
    
    setTimeout(() => {
      dragging = false;
      offsetY = 0;
      vp.classList.remove('player-dragging');
      if (!vp.classList.contains('active')) {
        vp.style.transform = '';
      }
    }, 50);
  };

  vp.addEventListener('touchend', finish, { passive: true });
  vp.addEventListener('touchcancel', finish, { passive: true });
}

function setupPlayerArtworkSwipeGestures() {
  const area = document.getElementById('aw');
  if (!area || area.dataset.swipeBound) return;
  area.dataset.swipeBound = '1';

  let startX = 0;
  let startY = 0;
  let dragging = false;
  let moved = false;
  const THRESHOLD = 50;

  function resetVisual() {
    area.classList.remove('is-swiping');
    area.style.transform = '';
    area.style.opacity = '';
  }

  function doSwipe(dir) {
    if (!window.T || !window.T.length || cur < 0) return;
    const len = window.T.length;
    const nextIdx = dir === 'left'
      ? (cur + 1) % len
      : (cur - 1 + len) % len;
    const outCls = dir === 'left' ? 'mini-swipe-out-left' : 'mini-swipe-out-right';
    const inCls = dir === 'left' ? 'mini-swipe-in-left' : 'mini-swipe-in-right';

    area.classList.add(outCls);
    area.addEventListener('animationend', function handler() {
      area.removeEventListener('animationend', handler);
      area.classList.remove(outCls);
      if (dir === 'left') {
        playNext();
      } else {
        play(nextIdx);
      }
      area.classList.add(inCls);
      area.addEventListener('animationend', function h2() {
        area.removeEventListener('animationend', h2);
        area.classList.remove(inCls);
      });
    });
  }

  area.addEventListener('touchstart', (e) => {
    if (!document.getElementById('vp')?.classList.contains('active') || !e.touches?.length) return;
    startX = e.touches[0].clientX;
    startY = e.touches[0].clientY;
    dragging = true;
    moved = false;
  }, { passive: true });

  area.addEventListener('touchmove', (e) => {
    if (!dragging || !e.touches?.length) return;
    const dx = e.touches[0].clientX - startX;
    const dy = e.touches[0].clientY - startY;
    if (Math.abs(dx) <= Math.abs(dy) + 6) return;
    moved = true;
    area.classList.add('is-swiping');
    area.style.transform = `translateX(${dx * 0.35}px)`;
    area.style.opacity = String(1 - Math.min(Math.abs(dx) / 180, 0.4));
    e.stopPropagation();
  }, { passive: true });

  const finish = (clientX) => {
    if (!dragging) return;
    dragging = false;
    const dx = clientX - startX;
    resetVisual();
    if (moved && Math.abs(dx) >= THRESHOLD) {
      doSwipe(dx < 0 ? 'left' : 'right');
    }
    moved = false;
  };

  area.addEventListener('touchend', (e) => {
    finish(e.changedTouches?.[0]?.clientX ?? startX);
  }, { passive: true });
  area.addEventListener('touchcancel', () => {
    finish(startX);
  }, { passive: true });
}

function getPlaylistLeadTrack(pl) {
  if (!pl) return null;
  if (pl.id === 'all') return T[0] || null;
  const ids = Array.isArray(pl.trackIds) ? pl.trackIds : [];
  for (const id of ids) {
    const track = trackById(id);
    if (track) return track;
  }
  return null;
}

function getPlaylistArtwork(pl) {
  if (pl?.id === 'favorites') {
    const favLead = (pl.trackIds || []).map(trackById).find(Boolean);
    return favLead?.art || null;
  }
  if (pl?.id === 'recent') {
    const recentLead = (pl.trackIds || []).map(trackById).find(Boolean);
    return recentLead?.art || null;
  }
  return getPlaylistLeadTrack(pl)?.art || null;
}

function getPlaylistSubtitle(pl, count) {
  const lead = getPlaylistLeadTrack(pl);
  const kind = pl?.id === 'favorites'
    ? 'Избранное'
    : pl?.id === 'recent'
      ? 'История'
      : 'Плейлист';
  const secondary = lead?.artist || `${count} треков`;
  return `${kind} • ${secondary}`;
}


function renderPlaylists() {
  const sec = document.getElementById('plsec');
  const list = document.getElementById('pll');
  if (!sec || !list) return;
  sec.style.display = 'block';
  list.innerHTML = '';
  P.forEach(pl => {
    if (pl.id === 'all') return;
    const el = document.createElement('div');
    el.className = 'plc' + (pl.id === activePlaylistId ? ' on' : '');
    const count = pl.id === 'all' ? T.length : (pl.trackIds || []).filter(id => !!trackById(id)).length;
    el.innerHTML = `<div class="pln">${esc(pl.name)}</div><div class="plm">${count} ${pl.id === 'all' ? 'треков' : 'в плейлисте'}</div>`;
    el.addEventListener('click', () => {
      activePlaylistId = pl.id;
      rl();
    });
    if (!pl.locked) {
      let lpTimer = null;
      el.addEventListener('touchstart', () => {
        lpTimer = setTimeout(() => {
          el.classList.add('lp-del');
          if (confirm(`Удалить плейлист «${pl.name}»?`)) {
            deletePlaylist(pl.id);
          } else {
            el.classList.remove('lp-del');
          }
        }, 600);
      }, { passive: true });
      el.addEventListener('touchend', () => { clearTimeout(lpTimer); }, { passive: true });
      el.addEventListener('touchcancel', () => { clearTimeout(lpTimer); }, { passive: true });
      el.addEventListener('touchmove', () => { clearTimeout(lpTimer); }, { passive: true });
    }
    list.appendChild(el);
  });
}

function renderPlaylistArtwork(pl, size = 52) {
  const ids = pl.id === 'all' ? T.slice(0,4).map(t=>t.id) : (pl.trackIds || []).slice(0,4);
  const arts = ids.map(id => trackById(id)?.art).filter(Boolean);
  if (arts.length >= 4) {
    return `<div class="pl-art-grid">
      ${arts.slice(0,4).map(a=>`<img src="${a}">`).join('')}
    </div>`;
  }
  if (arts.length) return `<img src="${arts[0]}" class="pl-art-single" style="width:100%;height:100%;object-fit:cover;border-radius:inherit">`;
  const iconSize = Math.round(size * 0.42);
  const favIcon = `<svg width="${iconSize}" height="${iconSize}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"/></svg>`;
  const noteIcon = `<svg width="${iconSize}" height="${iconSize}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>`;
  const recentIcon = `<svg width="${iconSize}" height="${iconSize}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 8v5l3 2"/><path d="M3.05 11a9 9 0 1 1 2.18 5.91"/><path d="M3 4v7h7"/></svg>`;
  const icon = pl.id === 'favorites' ? favIcon : pl.id === 'recent' ? recentIcon : noteIcon;
  return `<div class="pl-art-fallback" style="display:flex;align-items:center;justify-content:center;width:100%;height:100%;background:rgba(255,255,255,0.06);border-radius:inherit;color:rgba(255,255,255,0.55);">${icon}</div>`;
}

let openedPlaylistId = null;
let openedPlaylistFromSearch = false;

function openPlaylistView(plId, fromSearch) {
  openedPlaylistId = plId;
  openedPlaylistFromSearch = !!fromSearch;
  const pl = P.find(x => x.id === plId);
  if (!pl) return;
  activePlaylistId = plId;

  const vplEl = document.getElementById('vpl');
  if (pl.id === 'favorites' || pl.id === 'recent') {
    vplEl.classList.add('favorites-view');
    document.getElementById('vpl-artwork').style.display = 'none';
    document.getElementById('vpl-title').style.textAlign = 'left';
    document.getElementById('vpl-title').style.fontSize = '28px';
    document.getElementById('vpl-author').style.display = 'none';
  } else {
    vplEl.classList.remove('favorites-view');
    document.getElementById('vpl-artwork').style.display = '';
    document.getElementById('vpl-title').style.textAlign = 'center';
    document.getElementById('vpl-title').style.fontSize = '22px';
    document.getElementById('vpl-author').style.display = 'flex';
  }

  document.getElementById('vpl-artwork').innerHTML = renderPlaylistArtwork(pl, 220);
  document.getElementById('vpl-title').textContent = pl.name;
  
  if (pl.id !== 'favorites' && pl.id !== 'recent') {
    document.getElementById('vpl-author').innerHTML = `<span style="font-size:16px">👤</span> Liquify User`;
  }

  const count = pl.id === 'all' ? T.length : (pl.trackIds || []).filter(id => !!trackById(id)).length;
  const totalDuration = pl.id === 'all' ? T.reduce((a,b) => a + (b.duration||0), 0) : (pl.trackIds || []).reduce((a,b) => a + (trackById(b)?.duration||0), 0);
  document.getElementById('vpl-stats').textContent = (pl.id === 'favorites' || pl.id === 'recent') ? `${count} треков` : `${pl.saves || count || 0} треков • ${fmt(totalDuration)}`;

  // Plus button: if from search — add to library; if from library — already added (accent)
  const plusBtn = document.getElementById('vpl-act-like');
  if (plusBtn) {
    const isInLibrary = !pl.locked && P.some(x => x.id === pl.id);
    if (pl.id === 'favorites' || pl.id === 'recent') {
      plusBtn.style.display = 'none';
    } else {
      plusBtn.style.display = '';
      if (fromSearch && !isInLibrary) {
        // Not yet in library — show inactive plus
        plusBtn.style.color = 'rgba(255,255,255,0.5)';
        plusBtn.title = 'Добавить в медиатеку';
        plusBtn.onclick = async () => {
          // pl already created by importSCPlaylist, just save
          savePlaylistsCache();
          renderPlaylists();
          if (typeof window.sbEnsurePlaylistUploaded === 'function') {
            try {
              await window.sbEnsurePlaylistUploaded(pl, { immediate: true, hydrateTracks: true });
            } catch (e) {
              console.warn('[SB-SYNC] Immediate playlist upload failed:', e);
            }
          }
          plusBtn.style.color = 'rgba(167,139,250,1)';
          plusBtn.title = 'В медиатеке';
          scToast('Добавлено', pl.name, false);
          setTimeout(scToastHide, 2000);
        };
      } else {
        // In library — show accent plus
        plusBtn.style.color = 'rgba(167,139,250,1)';
        plusBtn.title = 'В медиатеке';
        plusBtn.onclick = null;
      }
    }
  }

  const shuffleBtn = document.getElementById('vpl-shuf');
  if (shuffleBtn) {
    shuffleBtn.className = 'vpl-act';
    shuffleBtn.title = 'Перемешать';
    shuffleBtn.style.cssText = 'background:none; border:none; color:rgba(255,255,255,0.55); padding:8px; cursor:pointer; display:flex; align-items:center; justify-content:center; border-radius:50%; transition:color .2s;';
    shuffleBtn.innerHTML = '<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 3 21 3 21 8"/><line x1="4" y1="20" x2="21" y2="3"/><polyline points="21 16 21 21 16 21"/><line x1="15" y1="15" x2="21" y2="21"/></svg>';
  }

  renderPlaylistTracks(plId, 'vpl-tracks');

  document.getElementById('vpl-play').onclick = () => {
    activePlaylistId = pl.id;
    shuf = false;
    rl();
    const entries = getVisibleTrackEntries();
    if (entries.length) play(entries[0].idx);
  };
  document.getElementById('vpl-shuf').onclick = () => {
    activePlaylistId = pl.id;
    shuf = true;
    rl();
    const entries = getVisibleTrackEntries();
    if (entries.length) play(entries[Math.floor(Math.random() * entries.length)].idx);
  };

  sv('vpl');
}

function closePlaylistView() {
  sv('l');
  openedPlaylistId = null;
}

function renderPlaylistTracks(plId, containerId) {
  const pl = P.find(x => x.id === plId);
  const container = document.getElementById(containerId);
  if (!pl || !container) return;
  container.innerHTML = '';
  
  const tracks = pl.id === 'all' 
    ? T.map((t, idx) => ({track: t, idx}))
    : (pl.trackIds || []).map(id => {
        const idx = T.findIndex(t => t.id === id);
        return idx >= 0 ? {track: T[idx], idx} : null;
      }).filter(Boolean);

  const frag = document.createDocumentFragment();
  tracks.forEach(({track, idx}) => {
    frag.appendChild(makeTrackEl(track, idx));
  });
  container.appendChild(frag);
}

function openArtistPage(user) {
  document.getElementById('artist-name').textContent = user.username;
  document.getElementById('artist-followers').textContent = user.followers_count ? fmtNum(user.followers_count) + ' подписчиков' : '';
  
  const avatar = user.avatar_url?.replace('-large', '-t500x500') || '';
  document.getElementById('artist-avatar').innerHTML = avatar ? `<img src="${avatar}">` : '<span>👤</span>';
  
  if (avatar) {
    document.getElementById('artist-hero').style.backgroundImage = `url(${avatar})`;
  }
  
  sv('vartist');
}

function closeArtistPage() {
  sv(lastNonPlayerView);
}

renderPlaylists = function () {
  const sec = document.getElementById('plsec');
  const list = document.getElementById('pll');
  if (!sec || !list) return;
  sec.style.display = 'block';
  list.innerHTML = '';
  P.forEach(pl => {
    if (pl.id === 'all') return; // не показываем "Все треки" в медиатеке
    const el = document.createElement('div');
    el.className = 'ti pl-row';
    const count = pl.id === 'all' ? T.length : (pl.trackIds || []).filter(id => !!trackById(id)).length;
    const artHtml = renderPlaylistArtwork(pl, 52);
    const subtitle = getPlaylistSubtitle(pl, count);
    // Determine if this is a SC playlist added to library
    const isScPlaylist = pl.source === 'soundcloud' || pl.sourceUrl;
    const isInLibrary = !pl.locked; // user-added playlists
    
    el.innerHTML = `
      <div class="ta" style="border-radius:10px; width:52px; height:52px; flex-shrink:0;">${artHtml}</div>
      <div class="tin">
        <div class="tn">${esc(pl.name)}</div>
        <div class="tar">${esc(subtitle)}</div>
      </div>`;
    
    el.addEventListener('click', () => {
      openPlaylistView(pl.id);
    });
    
    if (!pl.locked) {
      let lpTimer = null;
      el.addEventListener('touchstart', () => {
        lpTimer = setTimeout(() => {
          el.classList.add('lp-del');
          if (confirm(`Удалить плейлист «${pl.name}»?`)) {
            deletePlaylist(pl.id);
          } else {
            el.classList.remove('lp-del');
          }
        }, 600);
      }, { passive: true });
      el.addEventListener('touchend', () => { clearTimeout(lpTimer); }, { passive: true });
      el.addEventListener('touchcancel', () => { clearTimeout(lpTimer); }, { passive: true });
      el.addEventListener('touchmove', () => { clearTimeout(lpTimer); }, { passive: true });
    }
    list.appendChild(el);
  });
}



function openPlaylistCreate() {
  document.getElementById('pl-name').value = '';
  document.getElementById('pl-desc').value = '';
  document.getElementById('pl-create-ov').classList.add('show');
}

function closePlaylistCreate() {
  document.getElementById('pl-create-ov').classList.remove('show');
}

function createPlaylist(name, description, meta) {
  const nm = (name || '').trim();
  if (!nm) return null;
  const pl = {
    id: 'pl_' + Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
    name: nm,
    description: (description || '').trim(),
    trackIds: [],
    source: meta?.source || 'local',
    sourceUrl: meta?.sourceUrl || '',
    spotifyId: meta?.spotifyId || ''
  };
  P.push(pl);
  savePlaylistsCache();
  renderPlaylists();
  return pl;
}

function createPlaylistFromModal() {
  const name = document.getElementById('pl-name').value;
  const desc = document.getElementById('pl-desc').value;
  const pl = createPlaylist(name, desc);
  if (!pl) { alert('Введите название плейлиста'); return; }
  activePlaylistId = pl.id;
  closePlaylistCreate();
  
  if (window._pendingPlaylistTrackId) {
    const track = trackById(window._pendingPlaylistTrackId);
    if (track) setTimeout(() => openTrackPlaylistModalByTrack(track), 100);
    window._pendingPlaylistTrackId = null;
  }
  
  rl();
}

function openPlaylistCreateFromTrack() {
  closeTrackPlaylistModal();
  openPlaylistCreate();
  window._pendingPlaylistTrackId = trackPlaylistModalTrackId;
}

function openTrackPlaylistModalByTrack(track) {
  if (!track) return;
  ensureTrackId(track);
  trackPlaylistModalTrackId = track.id;
  document.getElementById('trk-pl-title').textContent = `Плейлисты: ${track.title}`;
  const box = document.getElementById('trk-pl-list');
  box.innerHTML = '';
  P.filter(x => !x.locked).forEach(pl => {
    const checked = (pl.trackIds || []).includes(track.id) ? 'checked' : '';
    const row = document.createElement('label');
    row.className = 'chk';
    row.innerHTML = `<input type="checkbox" data-pl="${pl.id}" ${checked}><span>${esc(pl.name)}</span>`;
    box.appendChild(row);
  });
  if (!box.children.length) {
    box.innerHTML = '<div class="emp" style="padding:20px 8px"><p>Сначала создай хотя бы один плейлист.</p></div>';
  }
  document.getElementById('trk-pl-ov').classList.add('show');
}

function closeTrackPlaylistModal() {
  document.getElementById('trk-pl-ov').classList.remove('show');
  trackPlaylistModalTrackId = null;
}

function saveTrackPlaylistModal() {
  const tid = trackPlaylistModalTrackId;
  if (!tid) { closeTrackPlaylistModal(); return; }
  const checks = [...document.querySelectorAll('#trk-pl-list input[type="checkbox"]')];
  P.forEach(pl => {
    if (pl.locked) return;
    const on = checks.find(c => c.dataset.pl === pl.id)?.checked;
    pl.trackIds = Array.from(new Set(pl.trackIds || []));
    const has = pl.trackIds.includes(tid);
    if (on && !has) pl.trackIds.push(tid);
    if (!on && has) pl.trackIds = pl.trackIds.filter(x => x !== tid);
  });
  savePlaylistsCache();
  renderPlaylists();
  rl();
  if (openedPlaylistId) {
    renderPlaylistTracks(openedPlaylistId, 'vpl-tracks');
    const openedPlaylist = P.find(x => x.id === openedPlaylistId);
    const stats = document.getElementById('vpl-stats');
    if (openedPlaylist && stats) {
      const count = openedPlaylist.id === 'all'
        ? T.length
        : (openedPlaylist.trackIds || []).filter(id => !!trackById(id)).length;
      const totalDuration = openedPlaylist.id === 'all'
        ? T.reduce((a, b) => a + (b.duration || 0), 0)
        : (openedPlaylist.trackIds || []).reduce((a, b) => a + (trackById(b)?.duration || 0), 0);
      stats.textContent = (openedPlaylist.id === 'favorites' || openedPlaylist.id === 'recent')
        ? `${count} треков`
        : `${openedPlaylist.saves || count || 0} треков • ${fmt(totalDuration)}`;
    }
  }
  closeTrackPlaylistModal();
}

saveTrackPlaylistModal = function() {
  const tid = trackPlaylistModalTrackId;
  if (!tid) { closeTrackPlaylistModal(); return; }
  const checks = [...document.querySelectorAll('#trk-pl-list input[type="checkbox"]')];
  P.forEach(pl => {
    if (pl.locked) return;
    const on = checks.find(c => c.dataset.pl === pl.id)?.checked;
    pl.trackIds = Array.from(new Set(pl.trackIds || []));
    const has = pl.trackIds.includes(tid);
    if (on && !has) pl.trackIds.push(tid);
    if (!on && has) pl.trackIds = pl.trackIds.filter(x => x !== tid);
  });
  savePlaylistsCache();
  renderPlaylists();
  rl();
  if (openedPlaylistId) {
    renderPlaylistTracks(openedPlaylistId, 'vpl-tracks');
    const openedPlaylist = P.find(x => x.id === openedPlaylistId);
    const stats = document.getElementById('vpl-stats');
    if (openedPlaylist && stats) {
      const count = openedPlaylist.id === 'all'
        ? T.length
        : (openedPlaylist.trackIds || []).filter(id => !!trackById(id)).length;
      const totalDuration = openedPlaylist.id === 'all'
        ? T.reduce((a, b) => a + (b.duration || 0), 0)
        : (openedPlaylist.trackIds || []).reduce((a, b) => a + (trackById(b)?.duration || 0), 0);
      stats.textContent = (openedPlaylist.id === 'favorites' || openedPlaylist.id === 'recent')
        ? `${count} \u0442\u0440\u0435\u043A\u043E\u0432`
        : `${openedPlaylist.saves || count || 0} \u0442\u0440\u0435\u043A\u043E\u0432 \u2022 ${fmt(totalDuration)}`;
    }
  }
  closeTrackPlaylistModal();
}

function canUseMediaSession() {
  return typeof navigator !== 'undefined' && 'mediaSession' in navigator;
}

function mediaArtworkFromTrack(t) {
  if (!t?.art) return [];
  const typeMatch = String(t.art).match(/^data:(image\/[a-zA-Z0-9.+-]+);/);
  const type = typeMatch ? typeMatch[1] : 'image/jpeg';
  return [
    { src: t.art, sizes: '96x96', type },
    { src: t.art, sizes: '128x128', type },
    { src: t.art, sizes: '192x192', type },
    { src: t.art, sizes: '256x256', type },
    { src: t.art, sizes: '384x384', type },
    { src: t.art, sizes: '512x512', type }
  ];
}

function setMediaAction(action, handler) {
  if (!canUseMediaSession()) return;
  try {
    navigator.mediaSession.setActionHandler(action, handler);
  } catch (e) { }
}

function syncMediaPlaybackState() {
  if (!canUseMediaSession()) return;
  try {
    navigator.mediaSession.playbackState = (playing && !aud.paused) ? 'playing' : 'paused';
  } catch (e) { }
}

function syncMediaSessionPosition(force) {
  if (!canUseMediaSession() || typeof navigator.mediaSession.setPositionState !== 'function') return;
  const now = Date.now();
  if (!force && now - msPosTick < 1000) return;
  msPosTick = now;
  const duration = Number(aud.duration);
  if (!isFinite(duration) || duration <= 0) return;
  try {
    navigator.mediaSession.setPositionState({
      duration,
      playbackRate: aud.playbackRate || 1,
      position: Math.max(0, Math.min(duration, Number(aud.currentTime) || 0))
    });
  } catch (e) { }
}

function syncMediaSessionTrack(t) {
  if (!canUseMediaSession() || !t || typeof MediaMetadata === 'undefined') return;
  navigator.mediaSession.metadata = new MediaMetadata({
    title: t.title || 'Unknown track',
    artist: t.artist || 'Unknown artist',
    album: t.album || '',
    artwork: mediaArtworkFromTrack(t)
  });
  syncMediaPlaybackState();
  syncMediaSessionPosition(true);
}

function initMediaSession() {
  if (!canUseMediaSession()) return;
  setMediaAction('play', () => aud.play());
  setMediaAction('pause', () => aud.pause());
  setMediaAction('nexttrack', playNext);
  setMediaAction('previoustrack', playPrev);
  setMediaAction('seekto', (details) => {
    if (!details || !isFinite(details.seekTime)) return;
    const duration = Number(aud.duration);
    const to = duration > 0 ? Math.max(0, Math.min(duration, details.seekTime)) : Math.max(0, details.seekTime);
    aud.currentTime = to;
    syncMediaSessionPosition(true);
  });
  setMediaAction('seekforward', (details) => {
    const step = (details && isFinite(details.seekOffset)) ? details.seekOffset : 10;
    const duration = Number(aud.duration);
    if (duration > 0) aud.currentTime = Math.min(duration, aud.currentTime + step);
    else aud.currentTime = Math.max(0, aud.currentTime + step);
    syncMediaSessionPosition(true);
  });
  setMediaAction('seekbackward', (details) => {
    const step = (details && isFinite(details.seekOffset)) ? details.seekOffset : 10;
    aud.currentTime = Math.max(0, aud.currentTime - step);
    syncMediaSessionPosition(true);
  });
  setMediaAction('stop', () => {
    aud.pause();
    aud.currentTime = 0;
    playing = false;
    upBtns();
    rl();
    syncMediaPlaybackState();
    syncMediaSessionPosition(true);
  });
}
initMediaSession();

function blobToDataUrl(blob) {
  return new Promise((res, rej) => {
    const r = new FileReader();
    r.onload = () => res(r.result);
    r.onerror = () => rej(new Error('read'));
    r.readAsDataURL(blob);
  });
}

/** Data URL for Android JNI (blob / http(s) / file URL → bytes on native). */
async function resolveTrackArtDataUrl(t) {
  if (!t || !t.art) return null;
  const a = String(t.art);
  if (a.startsWith('data:')) return a;
  try {
    const resp = await fetch(a);
    if (!resp.ok) return null;
    const blob = await resp.blob();
    return await blobToDataUrl(blob);
  } catch (_) {
    return null;
  }
}

async function syncNativeNowPlayingNotification() {
  const inv = getTauriInvoke();
  if (typeof inv !== 'function') return;
  const t = (cur >= 0 && T[cur]) ? T[cur] : null;
  if (!t) {
    try {
      await inv('android_clear_now_playing_notification');
    } catch (e) {
      console.warn('[notify] clear failed', e);
    }
    return;
  }
  try {
    const isPlayingNative = playing && !aud.paused;
    await inv('android_update_now_playing_notification', {
      title: t.title || 'Unknown Track',
      artist: t.artist || 'Unknown Artist',
      isPlaying: isPlayingNative,
      is_playing: isPlayingNative
    });
    const durationMs = isFinite(aud.duration) && aud.duration > 0
      ? Math.floor(aud.duration * 1000)
      : Math.floor((Number(t.duration) || 0) * 1000);
    const positionMs = Math.floor(Math.max(0, Number(aud.currentTime) || 0) * 1000);
    const art = await resolveTrackArtDataUrl(t);
    await inv('android_update_media_metadata', {
      title: t.title || 'Unknown Track',
      artist: t.artist || 'Unknown Artist',
      album: t.album || '',
      duration: durationMs,
      position: positionMs,
      artwork_base64: art,
      artworkBase64: art
    });
  } catch (e) {
    console.warn('[notify] update failed', e);
  }
}

// Mini player glass handled by global engine
window.addEventListener('resize', () => {
  if (wvd) drawWv(aud.currentTime, aud.duration);
});

//  NAVIGATION 
function sv(n) {
  const map = { h: 'vh', l: 'vl', p: 'vp', s: 'vs', sett: 'vsett', vpl: 'vpl', vartist: 'vartist', vprof: 'vprof' };
  const mini = document.getElementById('mini');
  const vp = document.getElementById('vp');
  if (vp) {
    vp.classList.remove('player-dragging');
    vp.style.removeProperty('--player-sheet-offset');
  }
  if (n === 'p') {
    openPlayerSheet();
    return;
  }
  if (!map[n]) return;
  if (vp) vp.classList.remove('active');
  setPlayerOverlayState(false);
  currentViewKey = n;
  lastNonPlayerView = n;
  document.querySelectorAll('.view').forEach(v => v.classList.remove('active'));
  document.getElementById(map[n]).classList.add('active');
  syncNavSelection(n);
  if (cur >= 0) {
    mini.classList.add('show');
  }
}

function refreshSearchResults() {
  const si = document.getElementById('si');
  if (!si) return;
  si.dispatchEvent(new Event('input'));
}

function removeTrackAtIndex(idx) {
  if (!Number.isInteger(idx) || idx < 0 || idx >= T.length) return;
  const removed = T[idx];
  const removedId = removed ? ensureTrackId(removed) : null;
  if (removedId) {
    P.forEach(pl => {
      if (pl.locked) return;
      if ((pl.trackIds || []).includes(removedId)) {
        pl.trackIds = pl.trackIds.filter(x => x !== removedId);
      }
    });
    savePlaylistsCache();
  }
  const wasCurrent = idx === cur;
  const wasPlaying = playing;
  if (wasCurrent) {
    aud.pause();
  }
  T.splice(idx, 1);
  if (idx < cur) cur -= 1;
  if (T.length === 0) {
    cur = -1;
    playing = false;
    aud.removeAttribute('src');
    aud.load();
    document.getElementById('pn').textContent = '—';
    document.getElementById('part').textContent = 'Выберите трек';
    document.getElementById('mint').textContent = '—';
    document.getElementById('minar').textContent = 'Выберите трек';
    document.getElementById('mini').classList.remove('show');
    upBtns();
    rl();
    refreshSearchResults();
    saveSoundCloudCache();
    syncNativeNowPlayingNotification();
    return;
  }
  if (wasCurrent) {
    const nextIdx = Math.min(idx, T.length - 1);
    if (wasPlaying) {
      play(nextIdx);
    } else {
      cur = nextIdx;
      const t = T[nextIdx];
      setAudSrcIfChanged(t.url);
      upUI(t);
      upMini(t);
      upBtns();
      rl();
      syncNativeNowPlayingNotification();
    }
  } else {
    rl();
    if (cur >= 0 && T[cur]) {
      upMini(T[cur]);
    }
    syncNativeNowPlayingNotification();
  }
  refreshSearchResults();
  saveSoundCloudCache();
}

function enableSwipeToDelete(el, getIndex) {
  let startX = 0;
  let deltaX = 0;
  let moving = false;
  let swiped = false;
  const DELETE_THRESHOLD = 100;
  el.style.touchAction = 'pan-y';

  el.addEventListener('touchstart', (e) => {
    if (!e.touches || !e.touches.length) return;
    startX = e.touches[0].clientX;
    deltaX = 0;
    moving = true;
    swiped = false;
    el.dataset.swiped = '0';
    el.style.transition = 'none';
  }, { passive: true });

  el.addEventListener('touchmove', (e) => {
    if (!moving || !e.touches || !e.touches.length) return;
    deltaX = e.touches[0].clientX - startX;
    if (deltaX > 0) deltaX = 0;
    if (Math.abs(deltaX) < 8) return;
    swiped = true;
    el.dataset.swiped = '1';
    const tx = Math.max(deltaX, -150);
    el.style.transform = `translateX(${tx}px)`;
    const ratio = Math.min(Math.abs(tx) / 150, 1);
    el.style.background = `rgba(239,68,68,${0.12 + ratio * 0.3})`;
  }, { passive: true });

  const finish = () => {
    if (!moving) return;
    moving = false;
    el.style.transition = 'transform .18s ease, background .18s ease';
    if (swiped && Math.abs(deltaX) >= DELETE_THRESHOLD) {
      el.style.transform = 'translateX(-120%)';
      setTimeout(() => removeTrackAtIndex(getIndex()), 120);
    } else {
      el.style.transform = '';
      el.style.background = '';
    }
    setTimeout(() => {
      el.style.transition = '';
      if (el.dataset.swiped === '1') el.dataset.swiped = '0';
    }, 220);
  };

  el.addEventListener('touchend', finish, { passive: true });
  el.addEventListener('touchcancel', finish, { passive: true });
}

//  FILE LOADING 
document.getElementById('fi').addEventListener('change', e => {
  const files = [...e.target.files];
  const audioFiles = files.filter(f => f.type.startsWith('audio/') || !f.name.toLowerCase().endsWith('.csv'));
  const csvFiles = files.filter(f => f.name.toLowerCase().endsWith('.csv'));

  if (audioFiles.length) hf(audioFiles);
  if (csvFiles.length) {
    csvFiles.forEach(f => {
      const reader = new FileReader();
      reader.onload = (ev) => importCsvData(ev.target.result, f.name.replace(/\.[^/.]+$/, ''));
      reader.readAsText(f);
    });
  }
  e.target.value = '';
});
const dz = document.getElementById('dz');
dz.addEventListener('dragover', e => { e.preventDefault(); dz.classList.add('drag'); });
dz.addEventListener('dragleave', () => dz.classList.remove('drag'));
dz.addEventListener('drop', e => {
  e.preventDefault(); dz.classList.remove('drag');
  hf([...e.dataTransfer.files].filter(f => f.type.startsWith('audio/')));
});

function hf(files) {
  Promise.all(files.map(lf)).then(() => {
    // Stamp insertion order for new tracks
    T.forEach((t, i) => { if (t._insertIdx == null) t._insertIdx = i; });
    rl();
    renderPlaylists();
  });
}

function syncSafeToInt(a, b, c, d) { return ((a & 0x7f) << 21) | ((b & 0x7f) << 14) | ((c & 0x7f) << 7) | (d & 0x7f); }
function readLatin1(bytes) { let s = ''; for (let i = 0; i < bytes.length; i++)s += String.fromCharCode(bytes[i]); return s; }
function decodeTextBytes(bytes, enc) {
  try {
    if (enc === 3) return new TextDecoder('utf-8').decode(bytes).replace(/\u0000/g, '').trim();
    if (enc === 1 || enc === 2) return new TextDecoder('utf-16').decode(bytes).replace(/\u0000/g, '').trim();
  } catch (e) { }
  return readLatin1(bytes).replace(/\u0000/g, '').trim();
}
function findTerm(bytes, start, enc) {
  if (enc === 1 || enc === 2) {
    for (let i = start; i < bytes.length - 1; i += 2) { if (bytes[i] === 0 && bytes[i + 1] === 0) return i; }
    return bytes.length;
  }
  for (let i = start; i < bytes.length; i++) { if (bytes[i] === 0) return i; }
  return bytes.length;
}
function parseId3TagsFromArrayBuffer(ab) {
  const u = new Uint8Array(ab);
  const out = { title: null, artist: null, album: null, artUrl: null };
  if (u.length < 10 || u[0] !== 0x49 || u[1] !== 0x44 || u[2] !== 0x33) return out;
  const ver = u[3];
  const tagSize = syncSafeToInt(u[6], u[7], u[8], u[9]);
  let off = 10;
  const end = Math.min(u.length, 10 + tagSize);
  while (off + 10 <= end) {
    const id = String.fromCharCode(u[off], u[off + 1], u[off + 2], u[off + 3]);
    if (!/^[A-Z0-9]{4}$/.test(id)) break;
    const sz = ver === 4 ? syncSafeToInt(u[off + 4], u[off + 5], u[off + 6], u[off + 7]) : ((u[off + 4] << 24) | (u[off + 5] << 16) | (u[off + 6] << 8) | u[off + 7]) >>> 0;
    if (!sz || off + 10 + sz > end) break;
    const f = u.slice(off + 10, off + 10 + sz);
    if ((id === 'TIT2' || id === 'TPE1' || id === 'TALB') && f.length > 1) {
      const txt = decodeTextBytes(f.slice(1), f[0]);
      if (id === 'TIT2') out.title = txt;
      if (id === 'TPE1') out.artist = txt;
      if (id === 'TALB') out.album = txt;
    } else if (id === 'APIC' && f.length > 10) {
      const enc = f[0];
      const mimeEnd = findTerm(f, 1, 0);
      const mime = readLatin1(f.slice(1, mimeEnd)) || 'image/jpeg';
      let p = mimeEnd + 1;
      p += 1;
      const descEnd = findTerm(f, p, enc);
      p = descEnd + ((enc === 1 || enc === 2) ? 2 : 1);
      if (p < f.length) {
        out.artUrl = URL.createObjectURL(new Blob([f.slice(p)], { type: mime }));
      }
    }
    off += 10 + sz;
  }
  return out;
}
function applyParsedTags(track, tags) {
  if (tags.title) track.title = tags.title;
  if (tags.artist) track.artist = tags.artist;
  if (tags.album) track.album = tags.album;
  if (tags.artUrl && !track.art) track.art = tags.artUrl;
}
function readTagsFromBlob(blob) {
  return new Promise(resolve => {
    if (typeof jsmediatags !== 'undefined') {
      jsmediatags.read(blob, {
        onSuccess(tag) {
          const g = tag.tags || {};
          const out = { title: g.title || null, artist: g.artist || null, album: g.album || null, artUrl: null };
          if (g.picture) {
            const data = new Uint8Array(g.picture.data || []);
            const mime = g.picture.format || 'image/jpeg';
            out.artUrl = URL.createObjectURL(new Blob([data], { type: mime }));
          }
          resolve(out);
        },
        onError() {
          blob.arrayBuffer().then(ab => resolve(parseId3TagsFromArrayBuffer(ab))).catch(() => resolve({}));
        }
      });
    } else {
      blob.arrayBuffer().then(ab => resolve(parseId3TagsFromArrayBuffer(ab))).catch(() => resolve({}));
    }
  });
}

function lf(file) {
  return new Promise(res => {
    const url = URL.createObjectURL(file);
    const t = {
      file, url, title: file.name.replace(/\.[^/.]+$/, '').replace(/^\d+[\s._-]+/, ''),
      artist: 'Неизвестный исполнитель', album: '', duration: 0, art: null, liked: false
    };
    ensureTrackId(t);
    const ta = new Audio(url);
    ta.addEventListener('loadedmetadata', () => { t.duration = ta.duration }, { once: true });
    readTagsFromBlob(file).then(tags => {
      applyParsedTags(t, tags || {});
      T.push(t); res(t);
    }).catch(() => { T.push(t); res(t); });
  });
}

//  RENDER

function makeTrackEl(t, i) {
  const el = document.createElement('div');
  el.className = 'ti' + (i === cur ? ' now' : '');
  el.dataset.idx = i;
  el.innerHTML = `
      <div class="ta">${t.art ? `<img src="${t.art}" alt="">` : '<span></span>'}</div>
      <div class="tin">
        <div class="tn">${esc(t.title)}</div>
        <div class="tar">${esc(t.artist)}</div>
      </div>
      <div class="rowact">
        <button class="tplus" title="В плейлисты"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg></button>
      ${i === cur && playing
      ? '<div class="eq"><span></span><span></span><span></span></div>'
      : `<div class="td">${fmt(t.duration)}</div>`}
      </div>`;
  el.querySelector('.tplus').addEventListener('click', (ev) => {
    ev.stopPropagation();
    openTrackPlaylistModalByTrack(t);
  });
  el.addEventListener('click', () => {
    if (el.dataset.swiped === '1') return;
    play(i);
  });
  enableSwipeToDelete(el, () => i);
  return el;
}

function syncPlayingUI() {
  ['tl', 'vpl-tracks'].forEach(listId => {
    const list = document.getElementById(listId);
    if (!list) return;
    list.querySelectorAll('.ti').forEach(el => {
      const i = parseInt(el.dataset.idx, 10);
      if (isNaN(i)) return;
      const isNow = i === cur;
      el.classList.toggle('now', isNow);
      const rowact = el.querySelector('.rowact');
      if (!rowact) return;
      const hasEq = rowact.querySelector('.eq');
      const shouldHaveEq = isNow && playing;
      if (shouldHaveEq && !hasEq) {
        const td = rowact.querySelector('.td');
        if (td) td.remove();
        rowact.insertAdjacentHTML('beforeend', '<div class="eq"><span></span><span></span><span></span></div>');
      } else if (!shouldHaveEq && hasEq) {
        hasEq.remove();
        const t = T[i];
        if (t) rowact.insertAdjacentHTML('beforeend', `<div class="td">${fmt(t.duration)}</div>`);
      }
    });
  });
}

function rl() {
  const list = document.getElementById('tl');
  const sr = document.getElementById('secrow');
  const dz = document.getElementById('dz');
  let vis = getVisibleTrackEntries();

  // Offline mode: filter to only show tracks that are cached offline or are local files
  if (offlineModeEnabled) {
    vis = vis.filter(({ track: t }) => t._offlineCached || (t.file && t.url && t.url.startsWith('blob:')));
  }

  const label = activePlaylistId === 'all' ? 'треков' : 'в плейлисте';
  document.getElementById('cbadge').textContent = `${vis.length} ${label}`;
  if (sr) sr.style.display = T.length ? 'flex' : 'none';

  // В медиатеке (#vl) треки не показываем — только плейлисты
  // Показываем dz только если нет вообще плейлистов пользовательских
  const isLibraryView = document.getElementById('vl')?.classList.contains('active');
  if (dz) {
    const hasUserPlaylists = P.filter(pl => !pl.locked).length > 0;
    if (isLibraryView) {
      dz.style.display = hasUserPlaylists ? 'none' : 'flex';
    } else {
      dz.style.display = T.length ? 'none' : 'flex';
    }
  }

  // В медиатеке не рендерим треки в #tl
  if (isLibraryView) {
    if (list) list.innerHTML = '';
  } else {
    // Render ALL tracks synchronously to prevent list jerking when clicking track 11+
    const frag = document.createDocumentFragment();
    vis.forEach(({ track: t, idx: i }) => frag.appendChild(makeTrackEl(t, i)));
    if (list) { list.innerHTML = ''; list.appendChild(frag); }
  }

  renderPlaylists();
}

//  PLAYBACK 
let scAudErrorTimer = null;
function normalizeMediaSrc(u) {
  if (!u) return '';
  try {
    return new URL(u, window.location.href || 'http://127.0.0.1').href;
  } catch {
    return String(u);
  }
}
function setAudSrcIfChanged(url) {
  if (!url) return true;
  const next = normalizeMediaSrc(url);
  const prev = normalizeMediaSrc(aud.getAttribute('src') || aud.src || '');
  if (prev === next) return false;
  aud.src = url;
  return true;
}

let offlineAudioDbPromise = null;
const offlineObjectUrlByTrackId = new Map();

function openOfflineAudioDb() {
  if (offlineAudioDbPromise) return offlineAudioDbPromise;
  offlineAudioDbPromise = new Promise((resolve, reject) => {
    try {
      const req = indexedDB.open(OFFLINE_AUDIO_DB, 1);
      req.onupgradeneeded = () => {
        const db = req.result;
        if (!db.objectStoreNames.contains(OFFLINE_AUDIO_STORE)) {
          db.createObjectStore(OFFLINE_AUDIO_STORE, { keyPath: 'id' });
        }
      };
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error || new Error('indexeddb'));
    } catch (err) {
      reject(err);
    }
  }).catch(err => {
    console.warn('[offline-audio] db open failed', err);
    offlineAudioDbPromise = null;
    return null;
  });
  return offlineAudioDbPromise;
}

function isNetworkAudioUrl(url) {
  return /^https?:\/\//i.test(String(url || '')) || /^http:\/\/127\.0\.0\.1:\d+\/stream\?/i.test(String(url || ''));
}

async function getOfflineCachedTrackUrl(track) {
  if (!track || !track.id) return null;
  if (offlineObjectUrlByTrackId.has(track.id)) {
    return offlineObjectUrlByTrackId.get(track.id);
  }
  const db = await openOfflineAudioDb();
  if (!db) return null;
  return await new Promise(resolve => {
    const tx = db.transaction(OFFLINE_AUDIO_STORE, 'readonly');
    const store = tx.objectStore(OFFLINE_AUDIO_STORE);
    const req = store.get(track.id);
    req.onsuccess = () => {
      const row = req.result;
      if (!row || !row.blob) { resolve(null); return; }
      const old = offlineObjectUrlByTrackId.get(track.id);
      if (old) URL.revokeObjectURL(old);
      const obj = URL.createObjectURL(row.blob);
      offlineObjectUrlByTrackId.set(track.id, obj);
      resolve(obj);
    };
    req.onerror = () => resolve(null);
  });
}

async function cacheTrackAudioForOffline(track, srcUrl) {
  if (!track || !track.id || !isNetworkAudioUrl(srcUrl)) return;
  const db = await openOfflineAudioDb();
  if (!db) return;
  try {
    const tx0 = db.transaction(OFFLINE_AUDIO_STORE, 'readonly');
    const hasReq = tx0.objectStore(OFFLINE_AUDIO_STORE).get(track.id);
    const exists = await new Promise(r => {
      hasReq.onsuccess = () => r(!!hasReq.result?.blob);
      hasReq.onerror = () => r(false);
    });
    if (exists) return;
    const resp = await fetch(srcUrl, { cache: 'force-cache' });
    if (!resp.ok) return;
    const blob = await resp.blob();
    if (!blob || blob.size < 1024) return;
    const tx = db.transaction(OFFLINE_AUDIO_STORE, 'readwrite');
    tx.objectStore(OFFLINE_AUDIO_STORE).put({
      id: track.id,
      title: track.title || '',
      artist: track.artist || '',
      mime: blob.type || 'audio/mpeg',
      size: blob.size,
      updatedAt: Date.now(),
      blob
    });
    tx.oncomplete = () => { track._offlineCached = true; };
  } catch (err) {
    console.warn('[offline-audio] cache failed', err);
  }
}

async function playCurrentTrackObject(t, epoch) {
  if (epoch !== undefined && epoch !== playEpoch) return;
  if (scAudErrorTimer) {
    clearTimeout(scAudErrorTimer);
    scAudErrorTimer = null;
  }
  let src = t.url;
  try {
    const cachedSrc = await getOfflineCachedTrackUrl(t);
    if (cachedSrc) src = cachedSrc;
  } catch (_) { }
  setAudSrcIfChanged(src);
  if (src === t.url) {
    void cacheTrackAudioForOffline(t, t.url);
  }
  syncMediaSessionTrack(t);
  playing = false;
  try { upUI(t); } catch (e) { console.error('upUI fail', e); }
  try { upMini(t); } catch (e) { console.error('upMini fail', e); }
  try { extractColor(t, epoch); } catch (e) { console.error('Color fail', e); }
  void analyzeWv(t);
  try { upBtns(); } catch (e) { console.error('upBtns fail', e); }
  syncPlayingUI();
  syncNativeNowPlayingNotification();
  // Save last playing track for restore on restart
  saveLastTrack(t);
  // Preload neighbor cover art for smooth mini-player transitions
  preloadNeighborCovers();
  // Pre-fetch next track stream URL for seamless transition
  prefetchNextTrackStream();
  try {
    await aud.play();
  } catch (err) {
    if (epoch === playEpoch) {
      playing = false;
      try { upBtns(); } catch (e) { }
      syncPlayingUI();
    }
    throw err;
  }
  if (epoch !== undefined && epoch !== playEpoch) return;
  playing = true;
  try { upBtns(); } catch (e) { console.error('upBtns fail', e); }
  syncPlayingUI();
  syncNativeNowPlayingNotification();
  pushRecentTrack(t);
  // Cache audio for offline playback after first listen
  if (src && isNetworkAudioUrl(src)) {
    void cacheTrackAudioForOffline(t, src);
  }
}

async function play(i) {
  if (!Number.isInteger(i) || i < 0 || i >= T.length) return;
  const t = T[i];
  if (!t) return;
  ensureTrackId(t);
  
  cur = i;
  playing = false;
  syncPlayingUI(); // Update UI instantly to show this track is selected (stops equalizer lines flickering)
  
  const epoch = ++playEpoch;
  if (t.isSpotify && (!t.url || !t.streamUrl)) {
    await spPlayFallback(t, epoch);
    return;
  }
  // PRE-REFRESH SoundCloud tracks to ensure URL is fresh before playback
  // Only do this if it's NOT offline cached (if offline, we already have the audio locally)
  if (t.isSoundCloud && !t._offlineCached) {
    try {
      const refreshed = await scRefreshTrackForPlayback(t, true);
      if (!refreshed) {
        console.warn('[SC] Pre-refresh failed for track, attempting playback anyway');
      }
    } catch (e) {
      console.error('[SC] Pre-refresh error:', e);
    }
    if (epoch !== playEpoch) return;
  }
  try {
    await playCurrentTrackObject(t, epoch);
  } catch (err) {
    if (epoch !== playEpoch) return;
    if (t.isSoundCloud) {
      await new Promise(r => setTimeout(r, 280));
      if (epoch !== playEpoch) return;
      try {
        if (!aud.paused && (aud.currentTime > 0.08 || aud.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA)) {
          playing = true;
          upBtns();
          syncPlayingUI();
          syncNativeNowPlayingNotification();
          return;
        }
      } catch (e) { }
      const refreshed = await scRefreshTrackForPlayback(t, true);
      if (refreshed) {
        try {
          const retryEpoch = ++playEpoch;
          const ti = T.indexOf(t);
          if (ti >= 0) cur = ti;
          await playCurrentTrackObject(t, retryEpoch);
        } catch (err2) {
          console.error('[SC] playback failed after refresh', err2);
        }
      } else {
        console.error('[SC] refresh failed, cannot play cached track', err);
      }
    } else if (t.isSpotify) {
      await spPlayFallback(t, epoch);
    } else {
      console.error(err);
    }
  }
}

async function spPlayFallback(t, epoch) {
  scToast('Spotify Player', `Searching stream for: ${t.title}`, true);
  try {
    const q = `${t.artist} - ${t.title}`;
    const clientId = await scGetClientId();
    const scSearchUrl = `https://api-v2.soundcloud.com/search/tracks?q=${encodeURIComponent(q)}&client_id=${clientId}&limit=5`;
    const text = await scFetch(scSearchUrl);
    if (!text) throw new Error('SoundCloud search failed');
    const json = JSON.parse(text);
    const results = json.collection || [];
    const match = results.find(r =>
      r.title.toLowerCase().includes(t.title.toLowerCase()) ||
      t.title.toLowerCase().includes(r.title.toLowerCase())
    ) || results[0];
    if (!match) {
      scToast('Spotify Player', 'No matching stream found', false);
      setTimeout(scToastHide, 3000);
      return;
    }
    const info = await scExtractTrackInfo(match, clientId);
    if (!info || !info.streamUrl) throw new Error('No stream URL for match');
    const port = await scGetProxyPort();
    const proxiedUrl = `http://127.0.0.1:${port}/stream?url=${encodeURIComponent(info.streamUrl)}`;
    const sid = t.spotifyId;
    if (sid) {
      t.spotifyUri = t.spotifyUri || `spotify:track:${sid}`;
      t.spotifyWebUrl = t.spotifyWebUrl || `https://open.spotify.com/track/${sid}`;
    }
    t.soundcloudWebUrl = info.permalinkUrl || t.soundcloudWebUrl || '';
    t.scPlaybackUrl = proxiedUrl;
    t.url = proxiedUrl;
    t.streamUrl = info.streamUrl;
    if (info.scTrackId) t.scTrackId = String(info.scTrackId);
    t.isSoundCloud = true;
    t.isSpotify = false;
    try { saveT(); } catch (e) { }
    const ti = T.indexOf(t);
    if (ti >= 0) cur = ti;
    const ep = epoch !== undefined ? epoch : ++playEpoch;
    await playCurrentTrackObject(t, ep);
    syncPlayingUI();
    scToastHide();
  } catch (e) {
    console.error('[Spotify Fallback] Error', e);
    scToast('Spotify Player', 'Failed to find stream', false);
    setTimeout(scToastHide, 3000);
  }
}
function togglePlay() {
  if (cur < 0) return;
  if (playing) { aud.pause(); playing = false; }
  else { aud.play(); playing = true; }
  upBtns(); syncPlayingUI();
}
function playNext() {
  const nextIdx = getNextPlaybackIndex();
  if (nextIdx < 0) return;
  play(nextIdx);
}
function playPrev() {
  if (!getPlaybackEntries().length) return;
  if (aud.currentTime > 3) { aud.currentTime = 0; return; }
  const prevIdx = getPrevPlaybackIndex();
  if (prevIdx < 0) return;
  play(prevIdx);
}

/** Android notification / headset → WebView (MainActivity.dispatchPlaybackJson). */
window.__lqNativePlayback = function (raw) {
  try {
    const o = (typeof raw === 'string') ? JSON.parse(raw) : raw;
    if (!o || !o.type) return;
    if (o.type === 'state') {
      if (o.isPlaying) {
        if (cur >= 0 && aud.paused) aud.play().catch(() => { });
        playing = true;
      } else {
        aud.pause();
        playing = false;
      }
      upBtns(); rl();
      void syncNativeNowPlayingNotification();
      syncMediaPlaybackState();
      syncMediaSessionPosition(true);
      return;
    }
    if (o.type === 'action') {
      const a = o.action;
      if (a === 'next') { playNext(); return; }
      if (a === 'previous') { playPrev(); return; }
      if (a === 'play') {
        if (cur < 0) return;
        aud.play().catch(() => { });
        playing = true;
        upBtns(); syncPlayingUI();
        void syncNativeNowPlayingNotification();
        syncMediaPlaybackState();
        syncMediaSessionPosition(true);
        return;
      }
      if (a === 'pause') {
        aud.pause();
        playing = false;
        upBtns(); syncPlayingUI();
        void syncNativeNowPlayingNotification();
        syncMediaPlaybackState();
        syncMediaSessionPosition(true);
        return;
      }
      if (a === 'seek' && o.value != null && aud) {
        const sec = Number(o.value) / 1000;
        if (isFinite(sec)) aud.currentTime = Math.max(0, sec);
        syncMediaSessionPosition(true);
        void syncNativeNowPlayingNotification();
      }
    }
  } catch (err) {
    console.warn('[native playback]', err);
  }
};

function toggleShuffle() { shuf = !shuf; document.getElementById('shb').classList.toggle('on', shuf); }
function toggleRepeat() { rep = (rep + 1) % 3; document.getElementById('reb').classList.toggle('on', rep > 0); }
function toggleLike() {
  if (cur < 0) return;
  const t = T[cur];
  t.liked = !t.liked;
  lkd = t.liked;

  const fav = P.find(x => x.id === 'favorites');
  if (fav) {
    fav.trackIds = Array.from(new Set(fav.trackIds || []));
    if (t.liked) {
      if (!fav.trackIds.includes(t.id)) fav.trackIds.push(t.id);
    } else {
      fav.trackIds = fav.trackIds.filter(id => id !== t.id);
    }
    savePlaylistsCache();
    saveT(); // Fix: Also persist the track list to keep the .liked property in sync
    renderPlaylists();
  }

  const b = document.getElementById('lkbtn');
  b.classList.toggle('on', lkd);
  const svg = b.querySelector('svg');
  // Boost feedback
  b.style.transform = 'scale(1.3)';
  setTimeout(() => b.style.transform = '', 150);

  svg.setAttribute('fill', lkd ? '#ef4444' : 'none');
  svg.setAttribute('stroke', lkd ? '#ef4444' : 'currentColor');
}

//  AUDIO EVENTS 
aud.addEventListener('timeupdate', () => {
  const p = aud.duration ? (aud.currentTime / aud.duration) * 100 : 0;
  document.getElementById('mf').style.width = p + '%';
  document.getElementById('tc').textContent = fmt(aud.currentTime);
  if (aud.duration) document.getElementById('tt').textContent = fmt(aud.duration);
  drawWv(aud.currentTime, aud.duration);
  syncMediaSessionPosition(false);
});
aud.addEventListener('ended', () => rep === 2 ? (aud.currentTime = 0, aud.play()) : playNext());
aud.addEventListener('play', () => { playing = true; upBtns(); syncPlayingUI(); syncMediaPlaybackState(); syncMediaSessionPosition(true); syncNativeNowPlayingNotification(); });
aud.addEventListener('pause', () => { playing = false; upBtns(); syncPlayingUI(); syncMediaPlaybackState(); syncMediaSessionPosition(true); syncNativeNowPlayingNotification(); });
aud.addEventListener('error', () => {
  if (cur < 0 || !T[cur]) return;
  const t = T[cur];
  if (!t.isSoundCloud) return;
  if (t.__scRecovering) return;
  if (scAudErrorTimer) clearTimeout(scAudErrorTimer);
  scAudErrorTimer = setTimeout(() => {
    scAudErrorTimer = null;
    if (cur < 0 || !T[cur] || T[cur] !== t) return;
    const err = aud.error;
    if (!err) return;
    if (err.code === 1) return;
    t.__scRecovering = true;
    scRefreshTrackForPlayback(t, true)
      .then(ok => {
        if (ok && cur >= 0 && T[cur] === t) {
          setAudSrcIfChanged(t.url);
          return aud.play().catch(() => { });
        }
        return null;
      })
      .finally(() => { t.__scRecovering = false; });
  }, 520);
});

//  UI 
function upUI(t) {
  document.getElementById('pn').textContent = t.title;
  document.getElementById('part').textContent = t.artist;
  const ai = document.getElementById('ai'), em = document.getElementById('aem');
  const aic = document.getElementById('ai-canvas'), emc = document.getElementById('aem-canvas');
  if (t.art) {
    em.style.display = 'none';
    if(emc) emc.style.display = 'none';
    ai.style.backgroundImage = 'none';
    if(aic) aic.style.backgroundImage = 'none';
    
    let img = ai.querySelector('img.cover-img');
    if (!img) {
      img = document.createElement('img');
      img.className = 'cover-img';
      img.style.cssText = 'position:absolute;inset:0;width:100%;height:100%;object-fit:cover;border-radius:inherit';
      ai.appendChild(img);
    }
    img.src = t.art;
    
    if(aic) {
      let imgc = aic.querySelector('img.cover-img');
      if (!imgc) {
        imgc = document.createElement('img');
        imgc.className = 'cover-img';
        imgc.style.cssText = 'position:absolute;inset:0;width:100%;height:100%;object-fit:cover;border-radius:inherit';
        aic.appendChild(imgc);
      }
      imgc.src = t.art;
    }
  } else {
    em.style.display = '';
    if(emc) emc.style.display = '';
    ai.style.backgroundImage = '';
    if(aic) aic.style.backgroundImage = '';
    
    const img = ai.querySelector('img.cover-img');
    if (img) img.remove();
    
    if(aic) {
      const imgc = aic.querySelector('img.cover-img');
      if (imgc) imgc.remove();
    }
  }
  lkd = t.liked || false;
  // Deep sync with favorites playlist just in case
  const fav = P.find(x => x.id === 'favorites');
  if (fav && (fav.trackIds || []).includes(t.id)) lkd = true;
  t.liked = lkd;

  const lb = document.getElementById('lkbtn');
  lb.classList.toggle('on', lkd);
  const svg = lb.querySelector('svg');
  svg.setAttribute('fill', lkd ? '#ef4444' : 'none');
  svg.setAttribute('stroke', lkd ? '#ef4444' : 'currentColor');
  syncMediaSessionTrack(t);
  void refreshPlayerCanvas(t);
}

function upMini(t) {
  document.getElementById('mint').textContent = t.title;
  document.getElementById('minar').textContent = t.artist;
  const ma = document.getElementById('mina'), me = document.getElementById('minem');
  if (t.art) {
    me.style.display = 'none';
    let img = ma.querySelector('img');
    if (!img) { img = document.createElement('img'); ma.appendChild(img); }
    img.src = t.art;
  } else {
    me.style.display = ''; const img = ma.querySelector('img'); if (img) img.remove();
  }
  const mini = document.getElementById('mini');
  const onPlayerView = document.getElementById('vp').classList.contains('active');
  mini.classList.toggle('show', !onPlayerView);
}

function upBtns() {
  const pbIcon = document.getElementById('pbi');
  if (pbIcon) {
    pbIcon.innerHTML = playing
      ? '<rect x="6" y="4" width="4" height="16" rx="1" fill="#000"/><rect x="14" y="4" width="4" height="16" rx="1" fill="#000"/>'
      : '<polygon points="5 3 19 12 5 21 5 3" fill="#000"/>';
  }
  document.getElementById('pb').classList.toggle('on', playing);
  document.getElementById('mpi').innerHTML = playing
    ? '<rect x="6" y="4" width="4" height="16" rx="1" fill="currentColor"/><rect x="14" y="4" width="4" height="16" rx="1" fill="currentColor"/>'
    : '<polygon points="5 3 19 12 5 21 5 3" fill="currentColor"/>';
}

//  WAVEFORM 
async function analyzeWv(t) {
  const reqId = ++waveformRequestId;
  const key = getWaveformKey(t);
  const cached = key ? waveformCache.get(key) : null;
  if (cached && cached.length) {
    wvd = cached;
    drawWv(aud.currentTime || 0, aud.duration);
    return;
  }

  wvd = buildWaveformPlaceholder(t, 90);
  drawWv(aud.currentTime || 0, aud.duration);

  const inv = getTauriInvoke();
  if (typeof inv === 'function' && t.path && !t.isSoundCloud) {
    try {
      const peaks = await inv('compute_waveform', { path: t.path, bars: 90 });
      if (reqId !== waveformRequestId || cur < 0 || T[cur] !== t) return;
      if (setWaveformData(t, peaks, true)) {
        drawWv(aud.currentTime || 0, aud.duration);
        return;
      }
    } catch (e) { }
  }
  try {
    if (!actx) actx = new (window.AudioContext || window.webkitAudioContext)();
    const ab = await (await fetch(t.url)).arrayBuffer();
    const buf = await actx.decodeAudioData(ab);
    if (reqId !== waveformRequestId || cur < 0 || T[cur] !== t) return;
    const ch = buf.getChannelData(0), n = 90, bs = Math.floor(ch.length / n), raw = [];
    for (let i = 0; i < n; i++) { let s = 0; for (let j = 0; j < bs; j++)s += Math.abs(ch[i * bs + j]); raw.push(s / bs); }
    if (!setWaveformData(t, raw, true)) return;
  } catch (e) {
    if (reqId !== waveformRequestId || cur < 0 || T[cur] !== t) return;
    setWaveformData(t, buildWaveformPlaceholder(t, 90), false);
  }
  if (reqId !== waveformRequestId || cur < 0 || T[cur] !== t) return;
  drawWv(aud.currentTime || 0, aud.duration);
}

function drawWv(ct, dur) {
  const cv = document.getElementById('wv');
  const ctx = cv.getContext('2d');
  const dpr = window.devicePixelRatio || 1;
  const W = cv.clientWidth, H = 64;
  cv.width = W * dpr; cv.height = H * dpr;
  ctx.scale(dpr, dpr); ctx.clearRect(0, 0, W, H);
  const data = wvd || Array(90).fill(.3);
  const n = data.length, bw = (W / n) * .62, gap = (W / n) * .38;
  const pb = dur ? Math.floor((ct / dur) * n) : 0;
  const ah = getComputedStyle(document.documentElement).getPropertyValue('--ah').trim() || '#a78bfa';
  for (let i = 0; i < n; i++) {
    const x = i * (W / n) + gap / 2, amp = data[i], bh = Math.max(3, amp * (H - 10)), y = (H - bh) / 2;
    ctx.beginPath();
    if (ctx.roundRect) ctx.roundRect(x, y, bw, bh, 2); else ctx.rect(x, y, bw, bh);
    ctx.fillStyle = i <= pb ? ah : 'rgba(255,255,255,0.18)';
    ctx.globalAlpha = i <= pb ? .92 : 1;
    ctx.fill(); ctx.globalAlpha = 1;
  }
}

document.getElementById('wv').addEventListener('click', e => {
  if (!aud.duration) return;
  const rc = e.currentTarget.getBoundingClientRect();
  aud.currentTime = ((e.clientX - rc.left) / rc.width) * aud.duration;
});

//  COLOR EXTRACTION (from theme.js: avgColor + boostColor) 
const HDR10_MAX = 1023;
const DEFAULT_HDR10 = [670, 558, 1003];
function toHdr10(v8) { return Math.round((Math.max(0, Math.min(255, v8)) / 255) * HDR10_MAX); }
function hdr10To8f(v10) { return (Math.max(0, Math.min(HDR10_MAX, v10)) / HDR10_MAX) * 255; }
function hdr10To8i(v10) { return Math.round(hdr10To8f(v10)); }

function extractColor(t, epoch) {
  if (!t.art) { setA10(...DEFAULT_HDR10); updBg(null); return; }
  updBg(t.art);
  const src = String(t.art);
  const img = new Image();
  const low = src.toLowerCase();
  if (low.startsWith('http://') || low.startsWith('https://')) img.crossOrigin = 'anonymous';
  img.onload = () => {
    if (epoch !== undefined && epoch !== playEpoch) return;
    try {
      const c = document.createElement('canvas'); c.width = c.height = 60;
      const cx = c.getContext('2d', { willReadFrequently: true });
      cx.drawImage(img, 0, 0, 60, 60);
      const d = cx.getImageData(0, 0, 60, 60).data;
      let r = 0, g = 0, b = 0, n = 0;
      for (let i = 0; i < d.length; i += 4) { if (d[i + 3] < 128) continue; r += d[i]; g += d[i + 1]; b += d[i + 2]; n++; }
      if (!n) { setA10(...DEFAULT_HDR10); return; }
      const [br, bg2, bb] = boost10(toHdr10(Math.round(r / n)), toHdr10(Math.round(g / n)), toHdr10(Math.round(b / n)), 1.9, 1.25);
      setA10(br, bg2, bb);
    } catch (e) {
      setA10(...DEFAULT_HDR10);
    }
  };
  img.onerror = () => {
    if (epoch !== undefined && epoch !== playEpoch) return;
    setA10(...DEFAULT_HDR10);
  };
  img.src = src;
}

// 10-bit version of Liquify boostColor math.
function boost10(r10, g10, b10, sm, lm) {
  let r = r10 / HDR10_MAX, g = g10 / HDR10_MAX, b = b10 / HDR10_MAX;
  const mx = Math.max(r, g, b), mn = Math.min(r, g, b);
  let h, s, l = (mx + mn) / 2;
  if (mx === mn) { h = s = 0; }
  else {
    const d = mx - mn;
    s = l > .5 ? d / (2 - mx - mn) : d / (mx + mn);
    if (mx === r) h = (g - b) / d + (g < b ? 6 : 0);
    else if (mx === g) h = (b - r) / d + 2;
    else h = (r - g) / d + 4;
    h /= 6;
  }
  s = Math.min(s * sm, 1); l = Math.min(l * lm, .76);
  function h2r(p, q, t) {
    if (t < 0) t += 1; if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
  }
  let nr, ng, nb;
  if (s === 0) { nr = ng = nb = l; }
  else {
    const q = l < .5 ? l * (1 + s) : l + s - l * s, p = 2 * l - q;
    nr = h2r(p, q, h + 1 / 3); ng = h2r(p, q, h); nb = h2r(p, q, h - 1 / 3);
  }
  return [
    Math.round(nr * HDR10_MAX),
    Math.round(ng * HDR10_MAX),
    Math.round(nb * HDR10_MAX)
  ];
}

function setA10(r10, g10, b10) {
  const r = hdr10To8f(r10), g = hdr10To8f(g10), b = hdr10To8f(b10);
  const ri = hdr10To8i(r10), gi = hdr10To8i(g10), bi = hdr10To8i(b10);
  const hex = '#' + [ri, gi, bi].map(v => v.toString(16).padStart(2, '0')).join('');
  const p3r = (r10 / HDR10_MAX).toFixed(6), p3g = (g10 / HDR10_MAX).toFixed(6), p3b = (b10 / HDR10_MAX).toFixed(6);
  document.documentElement.style.setProperty('--a', `${r.toFixed(3)},${g.toFixed(3)},${b.toFixed(3)}`);
  document.documentElement.style.setProperty('--bg-a18-p3', `color(display-p3 ${p3r} ${p3g} ${p3b} / .18)`);
  document.documentElement.style.setProperty('--bg-a11-p3', `color(display-p3 ${p3r} ${p3g} ${p3b} / .11)`);
  document.documentElement.style.setProperty('--ah', hex);
  const vl = document.getElementById('vol');
  vl.style.background = `linear-gradient(to right,${hex} ${vl.value}%,rgba(255,255,255,.15) ${vl.value}%)`;
  document.getElementById('ac').style.boxShadow =
    `0 24px 80px rgba(0,0,0,.65),0 0 72px rgba(${r.toFixed(3)},${g.toFixed(3)},${b.toFixed(3)},.28)`;
  document.getElementById('mini').style.boxShadow =
    `0 0 36px rgba(${r.toFixed(3)},${g.toFixed(3)},${b.toFixed(3)},.14)`;
}

function setA(r, g, b) { setA10(toHdr10(r), toHdr10(g), toHdr10(b)); }

function updBg(url) {
  const el = document.getElementById('bga');
  if (url) { el.style.backgroundImage = `url(${url})`; el.classList.add('on'); }
  else el.classList.remove('on');
}

//  VOLUME 
document.getElementById('vol').addEventListener('input', e => {
  aud.volume = e.target.value / 100;
  const h = getComputedStyle(document.documentElement).getPropertyValue('--ah').trim() || '#a78bfa';
  e.target.style.background = `linear-gradient(to right,${h} ${e.target.value}%,rgba(255,255,255,.15) ${e.target.value}%)`;
});

const SEARCH_HISTORY_KEY = 'liquify_search_history_v2_tracks';
let searchHistory = []; // Array of {id, title, artist, art, isSoundCloud, scTrackId, ...}

function loadSearchHistory() {
  try {
    const raw = localStorage.getItem(SEARCH_HISTORY_KEY);
    if (raw) searchHistory = JSON.parse(raw) || [];
  } catch(e) { searchHistory = []; }
}
loadSearchHistory();

function addTrackToSearchHistory(track) {
  if (!track) return;
  ensureTrackId(track);
  // Remove duplicate
  searchHistory = searchHistory.filter(t => t.id !== track.id);
  searchHistory.unshift({ 
    id: track.id, 
    title: track.title, 
    artist: track.artist, 
    art: track.art || null,
    isSoundCloud: track.isSoundCloud || false,
    scTrackId: track.scTrackId || null,
    streamUrl: track.streamUrl || null,
    url: track.url || null,
    duration: track.duration || 0,
    album: track.album || ''
  });
  searchHistory = searchHistory.slice(0, 20);
  try { localStorage.setItem(SEARCH_HISTORY_KEY, JSON.stringify(searchHistory)); } catch(e) {}
}

function addToSearchHistory(query) {
  // Legacy stub - do nothing (history now track-based)
}

function renderSearchHistory() {
  const shist = document.getElementById('shist');
  const shistList = document.getElementById('shist-list');
  const res = document.getElementById('sr');
  const emp = document.getElementById('se');
  const sclear = document.getElementById('sclear');
  
  if (document.getElementById('si').value.trim()) {
    shist.style.display = 'none';
    sclear.style.display = 'block';
    return;
  }
  
  sclear.style.display = 'none';
  
  if (!searchHistory.length) {
    shist.style.display = 'none';
    res.innerHTML = '';
    emp.style.display = '';
    return;
  }
  
  shist.style.display = 'block';
  emp.style.display = 'none';
  res.innerHTML = '';
  
  shistList.innerHTML = '';
  searchHistory.forEach(ht => {
    const el = document.createElement('div');
    el.className = 'ti sh-track-item';
    el.innerHTML = `
      <div class="ta">${ht.art ? `<img src="${ht.art}" alt="">` : '<span>🎵</span>'}</div>
      <div class="tin">
        <div class="tn">${esc(ht.title)}</div>
        <div class="tar">${esc(ht.artist)}</div>
      </div>
      <button class="sh-item-del" title="Убрать" style="background:none;border:none;color:rgba(255,255,255,0.4);padding:8px;cursor:pointer;font-size:14px;">✕</button>
    `;
    el.querySelector('.sh-item-del').addEventListener('click', (e) => {
      e.stopPropagation();
      searchHistory = searchHistory.filter(t => t.id !== ht.id);
      try { localStorage.setItem(SEARCH_HISTORY_KEY, JSON.stringify(searchHistory)); } catch(e2) {}
      renderSearchHistory();
    });
    el.addEventListener('click', (e) => {
      if (e.target.classList.contains('sh-item-del')) return;
      // Play this track - find in T or play by stored info
      const existing = T.find(t => t.id === ht.id);
      if (existing) {
        const idx = T.indexOf(existing);
        play(idx);
      } else if (ht.isSoundCloud && ht.url) {
        // Re-add to T and play
        const restored = { ...ht, liked: false };
        ensureTrackId(restored);
        T.push(restored);
        rl();
        play(T.length - 1);
      }
    });
    shistList.appendChild(el);
  });
  
  const clearBtn = document.createElement('div');
  clearBtn.style.textAlign = 'center';
  clearBtn.style.marginTop = '8px';
  clearBtn.innerHTML = `<button class="shist-clear" onclick="clearSearchHistory()">Очистить историю</button>`;
  shistList.appendChild(clearBtn);
}

function clearSearchHistory() {
  searchHistory = [];
  localStorage.removeItem(SEARCH_HISTORY_KEY);
  renderSearchHistory();
}

function clearSearch() {
  const si = document.getElementById('si');
  si.value = '';
  si.dispatchEvent(new Event('input'));
  si.focus();
}

function setupSearchCategoryChips() {
  const view = document.getElementById('vs');
  const header = view ? view.querySelector('.shd') : null;
  if (!view || !header || view.querySelector('.search-chips')) return;
  const chips = document.createElement('div');
  chips.className = 'lib-chips search-chips';
  chips.innerHTML = `
    <button class="lib-chip active" id="search-chip-tracks" onclick="setSearchCategory('tracks')">Треки</button>
    <button class="lib-chip" id="search-chip-playlists" onclick="setSearchCategory('playlists')">Плейлисты</button>
    <button class="lib-chip" id="search-chip-artists" onclick="setSearchCategory('artists')">Исполнители</button>
  `;
  header.insertAdjacentElement('afterend', chips);
}

function setSearchCategory(kind) {
  searchCategory = kind;
  ['tracks', 'playlists', 'artists'].forEach((name) => {
    const chip = document.getElementById(`search-chip-${name}`);
    if (!chip) return;
    chip.classList.toggle('active', name === kind);
  });
  refreshSearchResults();
}

function renderSearchArtistResult(res, artistName, tracks) {
  const el = document.createElement('div');
  el.className = 'ti sc-artist-item';
  const lead = tracks.find(t => t.art) || tracks[0];
  el.innerHTML = `
    <div class="ta ta--round" style="border-radius:50%">
      ${lead && lead.art ? `<img src="${lead.art}" alt="" style="border-radius:50%">` : '<span style="border-radius:50%">👤</span>'}
    </div>
    <div class="tin">
      <div class="tn">${esc(artistName)}</div>
      <div class="tar">${tracks.length} треков в медиатеке</div>
    </div>
  `;
  el.addEventListener('click', () => {
    const si = document.getElementById('si');
    if (!si) return;
    si.value = artistName;
    setSearchCategory('tracks');
    si.dispatchEvent(new Event('input'));
  });
  res.appendChild(el);
}

function renderLocalSearchResults(q, res) {
  if (searchCategory === 'playlists') {
    const found = P.filter(pl => pl.id !== 'all' && (
      String(pl.name || '').toLowerCase().includes(q) ||
      String(pl.description || '').toLowerCase().includes(q)
    ));
    res.innerHTML = '';
    if (!found.length) {
      res.innerHTML = '<div class="emp"><h3>Ничего не найдено</h3><p>В вашей медиатеке нет таких плейлистов</p></div>';
      return;
    }
    found.forEach(pl => {
      const el = document.createElement('div');
      el.className = 'ti pl-row';
      const count = (pl.trackIds || []).filter(id => !!trackById(id)).length;
      const artHtml = renderPlaylistArtwork(pl, 52);
      const subtitle = getPlaylistSubtitle(pl, count);
      el.innerHTML = `
        <div class="ta" style="border-radius:10px; width:52px; height:52px; flex-shrink:0;">${artHtml}</div>
        <div class="tin">
          <div class="tn">${esc(pl.name)}</div>
          <div class="tar">${esc(subtitle)}</div>
        </div>`;
      el.addEventListener('click', () => openPlaylistView(pl.id));
      res.appendChild(el);
    });
    return;
  }

  if (searchCategory === 'artists') {
    const byArtist = new Map();
    T.forEach(t => {
      const artist = String(t.artist || '').trim();
      if (!artist || !artist.toLowerCase().includes(q)) return;
      if (!byArtist.has(artist)) byArtist.set(artist, []);
      byArtist.get(artist).push(t);
    });
    const found = Array.from(byArtist.entries()).sort((a, b) => b[1].length - a[1].length);
    res.innerHTML = '';
    if (!found.length) {
      res.innerHTML = '<div class="emp"><h3>Ничего не найдено</h3><p>В вашей медиатеке нет таких исполнителей</p></div>';
      return;
    }
    found.forEach(([artistName, tracks]) => renderSearchArtistResult(res, artistName, tracks));
    return;
  }

  const found = T.filter(t =>
    t.title.toLowerCase().includes(q) || t.artist.toLowerCase().includes(q) || t.album.toLowerCase().includes(q));
  res.innerHTML = '';
  if (!found.length) {
    res.innerHTML = '<div class="emp"><h3>Ничего не найдено</h3><p>В вашей медиатеке нет таких треков</p></div>'; return;
  }
  found.forEach(t => {
    const i = T.indexOf(t), el = document.createElement('div');
    el.className = 'ti' + (i === cur ? ' now' : '');
    el.innerHTML = `
      <div class="ta">${t.art ? `<img src="${t.art}" alt="">` : '<span></span>'}</div>
      <div class="tin"><div class="tn">${esc(t.title)}${t.liked ? ' <span style="color:#ef4444;font-size:12px">❤</span>' : ''}</div><div class="tar">${esc(t.artist)}</div></div>
      <div class="rowact"><button class="tplus" title="В плейлисты">+</button><div class="td">${fmt(t.duration)}</div></div>`;
    el.querySelector('.tplus').addEventListener('click', (ev) => {
      ev.stopPropagation();
      openTrackPlaylistModalByTrack(t);
    });
    el.addEventListener('click', () => {
      if (el.dataset.swiped === '1') return;
      addTrackToSearchHistory(t);
      play(i);
    });
    enableSwipeToDelete(el, () => i);
    res.appendChild(el);
  });
}

function toggleSearchProvider() {
  const btn = document.getElementById('provider-toggle');
  btn.classList.add('transitioning');
  setTimeout(() => {
    // Cycle: local → spotify → soundcloud → local
    const cycle = ['local', 'spotify', 'soundcloud'];
    const next = cycle[(cycle.indexOf(searchProvider) + 1) % cycle.length];
    // Skip spotify if no token
    searchProvider = (next === 'spotify' && !spotifyToken) ? 'soundcloud' : next;

    const cls = { local: 'lc', spotify: 'sp', soundcloud: 'sc' }[searchProvider];
    btn.className = `provider-btn ${cls} transitioning`;

    const iconLc = btn.querySelector('.icon-lc');
    const iconSp = btn.querySelector('.icon-sp');
    const iconSc = btn.querySelector('.icon-sc');
    if (iconLc) iconLc.style.display = searchProvider === 'local'      ? '' : 'none';
    if (iconSp) iconSp.style.display = searchProvider === 'spotify'    ? '' : 'none';
    if (iconSc) iconSc.style.display = searchProvider === 'soundcloud' ? '' : 'none';

    const si = document.getElementById('si');
    const placeholders = { local: 'Поиск в медиатеке...', spotify: 'Поиск в Spotify...', soundcloud: 'Поиск в SoundCloud...' };
    si.placeholder = placeholders[searchProvider];
    si.dispatchEvent(new Event('input'));
  }, 150);
  setTimeout(() => { btn.classList.remove('transitioning'); }, 300);
}

document.getElementById('si').addEventListener('focus', e => {
  if (!e.target.value.trim()) {
    renderSearchHistory();
  }
});

document.getElementById('si').addEventListener('input', e => {
  const q = e.target.value.toLowerCase().trim();
  const res = document.getElementById('sr'), emp = document.getElementById('se');
  
  if (!q) { 
    renderSearchHistory();
    return; 
  }
  
  document.getElementById('shist').style.display = 'none';
  document.getElementById('sclear').style.display = 'block';
  emp.style.display = 'none';

  if (searchProvider === 'local') {
    renderLocalSearchResults(q, res);
  } else if (searchProvider === 'spotify') {
    clearTimeout(searchDebounce);
    res.innerHTML = '<div class="s-loading">Поиск в Spotify...</div>';
    searchDebounce = setTimeout(() => spDoSearch(q), 500);
  } else {
    // SoundCloud Search
    clearTimeout(searchDebounce);
    res.innerHTML = '<div class="s-loading">Поиск в SoundCloud...</div>';
    searchDebounce = setTimeout(() => scDoSearch(q), 600);
  }
});

// ── Spotify Search ────────────────────────────────────────────────────────────
async function spDoSearch(q) {
  const res = document.getElementById('sr');
  if (!spotifyToken) {
    res.innerHTML = '<div class="emp"><h3>Нет Spotify токена</h3><p>Подключитесь к Spotify в настройках</p></div>';
    return;
  }
  try {
    const typeMap = { tracks: 'track', playlists: 'playlist', artists: 'artist' };
    const spType = typeMap[searchCategory] || 'track';
    const url = `https://api.spotify.com/v1/search?q=${encodeURIComponent(q)}&type=${spType}&limit=25&market=from_token`;
    const resp = await fetch(url, { headers: { Authorization: `Bearer ${spotifyToken}` } });
    if (!resp.ok) {
      if (resp.status === 401) {
        res.innerHTML = '<div class="emp"><h3>Токен истёк</h3><p>Переподключитесь к Spotify</p></div>';
      } else {
        res.innerHTML = `<div class="emp"><h3>Ошибка ${resp.status}</h3><p>Не удалось выполнить поиск</p></div>`;
      }
      return;
    }
    const data = await resp.json();
    res.innerHTML = '';

    if (searchCategory === 'tracks') {
      const items = data.tracks?.items || [];
      if (!items.length) {
        res.innerHTML = '<div class="emp"><h3>Ничего не найдено</h3><p>Spotify не нашёл треков по этому запросу</p></div>';
        return;
      }
      items.forEach(item => {
        const art = item.album?.images?.[0]?.url || '';
        const artists = item.artists?.map(a => a.name).join(', ') || '';
        const dur = Math.round((item.duration_ms || 0) / 1000);
        const inLib = T.some(t => t.spotifyId === item.id);
        const el = document.createElement('div');
        el.className = 'ti';
        el.innerHTML = `
          <div class="ta">${art ? `<img src="${art}" alt="">` : '<span></span>'}</div>
          <div class="tin">
            <div class="tn">${esc(item.name)} <span class="sp-badge">SP</span>${inLib ? ' <span style="color:#a78bfa;font-size:11px">✓</span>' : ''}</div>
            <div class="tar">${esc(artists)} · ${esc(item.album?.name || '')}</div>
          </div>
          <div class="rowact">
            <button class="tplus sp-add" title="${inLib ? 'В медиатеке' : 'Добавить в медиатеку'}" style="${inLib ? 'color:rgba(167,139,250,1)' : ''}">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
              </svg>
            </button>
            <div class="td">${fmt(dur)}</div>
          </div>`;
        el.querySelector('.tplus').addEventListener('click', async (ev) => {
          ev.stopPropagation();
          const btn = ev.currentTarget;
          if (inLib) { openTrackPlaylistModalByTrack(T.find(t => t.spotifyId === item.id)); return; }
          btn.style.opacity = '0.4';
          btn.style.pointerEvents = 'none';
          try {
            await spAddTrackToLibrary(item);
            btn.style.color = 'rgba(167,139,250,1)';
            btn.title = 'В медиатеке';
          } catch(e) {
            console.error('[SP] Add track failed:', e);
          } finally {
            btn.style.opacity = '';
            btn.style.pointerEvents = '';
          }
        });
        el.addEventListener('click', (e) => {
          if (e.target.closest('.tplus')) return;
          // Play from Spotify — добавляем трек если его нет и воспроизводим
          spPlayTrack(item);
        });
        res.appendChild(el);
      });

    } else if (searchCategory === 'artists') {
      const items = data.artists?.items || [];
      if (!items.length) {
        res.innerHTML = '<div class="emp"><h3>Ничего не найдено</h3><p>Spotify не нашёл исполнителей</p></div>';
        return;
      }
      items.forEach(item => {
        const img = item.images?.[0]?.url || '';
        const followers = item.followers?.total ? `${(item.followers.total / 1000).toFixed(0)}K слушателей` : '';
        const el = document.createElement('div');
        el.className = 'ti';
        el.innerHTML = `
          <div class="ta" style="border-radius:50%">${img ? `<img src="${img}" alt="" style="border-radius:50%">` : '<span></span>'}</div>
          <div class="tin">
            <div class="tn">${esc(item.name)} <span class="sp-badge">SP</span></div>
            <div class="tar">${esc(followers)}</div>
          </div>`;
        el.addEventListener('click', () => spOpenArtist(item.id, item.name));
        res.appendChild(el);
      });

    } else if (searchCategory === 'playlists') {
      const items = data.playlists?.items || [];
      if (!items.length) {
        res.innerHTML = '<div class="emp"><h3>Ничего не найдено</h3><p>Spotify не нашёл плейлистов</p></div>';
        return;
      }
      items.forEach(item => {
        if (!item) return;
        const img = item.images?.[0]?.url || '';
        const owner = item.owner?.display_name || '';
        const count = item.tracks?.total ?? '';
        const el = document.createElement('div');
        el.className = 'ti pl-row';
        el.innerHTML = `
          <div class="ta" style="border-radius:10px">${img ? `<img src="${img}" alt="">` : '<span></span>'}</div>
          <div class="tin">
            <div class="tn">${esc(item.name)} <span class="sp-badge">SP</span></div>
            <div class="tar">${esc(owner)}${count ? ` · ${count} треков` : ''}</div>
          </div>`;
        el.addEventListener('click', () => spOpenPlaylist(item.id, item.name, img));
        res.appendChild(el);
      });
    }
  } catch(e) {
    console.error('[Spotify Search] Error:', e);
    res.innerHTML = `<div class="emp"><h3>Ошибка поиска</h3><p>${esc(String(e))}</p></div>`;
  }
}

// Добавить трек из Spotify в медиатеку
async function spAddTrackToLibrary(spItem) {
  const art = spItem.album?.images?.[0]?.url || '';
  const artists = spItem.artists?.map(a => a.name).join(', ') || '';
  const dur = Math.round((spItem.duration_ms || 0) / 1000);
  const track = {
    id: 'sp_' + spItem.id,
    spotifyId: spItem.id,
    title: spItem.name,
    artist: artists,
    album: spItem.album?.name || '',
    art,
    duration: dur,
    liked: false,
    src: '', // нет локального файла
  };
  T.push(track);
  saveT();
  rl();
  return track;
}

// Воспроизвести трек из Spotify (добавляет в библиотеку если нет)
async function spPlayTrack(spItem) {
  const existing = T.findIndex(t => t.spotifyId === spItem.id);
  if (existing !== -1) { play(existing); return; }
  const track = await spAddTrackToLibrary(spItem);
  const idx = T.indexOf(track);
  if (idx !== -1) play(idx);
}

// Открыть страницу артиста из Spotify
async function spOpenArtist(artistId, artistName) {
  if (!spotifyToken) return;
  try {
    const [topResp, albumsResp] = await Promise.all([
      fetch(`https://api.spotify.com/v1/artists/${artistId}/top-tracks?market=from_token`, { headers: { Authorization: `Bearer ${spotifyToken}` } }),
      fetch(`https://api.spotify.com/v1/artists/${artistId}/albums?limit=10&include_groups=album,single&market=from_token`, { headers: { Authorization: `Bearer ${spotifyToken}` } }),
    ]);
    const topData   = topResp.ok   ? await topResp.json()   : { tracks: [] };
    const albumData = albumsResp.ok ? await albumsResp.json() : { items: [] };
    const res = document.getElementById('sr');
    res.innerHTML = `<div style="padding:12px 16px 4px;font-size:18px;font-weight:700">${esc(artistName)}</div>`;
    if (topData.tracks?.length) {
      const hdr = document.createElement('div');
      hdr.className = 'sc-sec-tit';
      hdr.style.cssText = 'font-size:14px;font-weight:600;padding:8px 16px 4px;opacity:0.6';
      hdr.textContent = 'Популярные треки';
      res.appendChild(hdr);
      topData.tracks.slice(0, 5).forEach(item => {
        const art = item.album?.images?.[0]?.url || '';
        const dur = Math.round((item.duration_ms || 0) / 1000);
        const el = document.createElement('div');
        el.className = 'ti';
        el.innerHTML = `
          <div class="ta">${art ? `<img src="${art}" alt="">` : '<span></span>'}</div>
          <div class="tin">
            <div class="tn">${esc(item.name)} <span class="sp-badge">SP</span></div>
            <div class="tar">${esc(item.album?.name || '')}</div>
          </div>
          <div class="rowact"><div class="td">${fmt(dur)}</div></div>`;
        el.addEventListener('click', () => spPlayTrack(item));
        res.appendChild(el);
      });
    }
    if (albumData.items?.length) {
      const hdr2 = document.createElement('div');
      hdr2.className = 'sc-sec-tit';
      hdr2.style.cssText = 'font-size:14px;font-weight:600;padding:12px 16px 4px;opacity:0.6';
      hdr2.textContent = 'Альбомы';
      res.appendChild(hdr2);
      albumData.items.forEach(album => {
        const img = album.images?.[0]?.url || '';
        const year = album.release_date?.slice(0, 4) || '';
        const el = document.createElement('div');
        el.className = 'ti pl-row';
        el.innerHTML = `
          <div class="ta" style="border-radius:10px">${img ? `<img src="${img}" alt="">` : '<span></span>'}</div>
          <div class="tin">
            <div class="tn">${esc(album.name)} <span class="sp-badge">SP</span></div>
            <div class="tar">${year ? year + ' · ' : ''}${album.album_type === 'single' ? 'Сингл' : 'Альбом'}</div>
          </div>`;
        el.addEventListener('click', () => spOpenAlbum(album.id, album.name, img));
        res.appendChild(el);
      });
    }
  } catch(e) {
    console.error('[SP] Artist page error:', e);
  }
}

// Открыть плейлист из Spotify
async function spOpenPlaylist(playlistId, name, img) {
  if (!spotifyToken) return;
  const res = document.getElementById('sr');
  res.innerHTML = `<div class="s-loading">Загрузка плейлиста...</div>`;
  try {
    const resp = await fetch(`https://api.spotify.com/v1/playlists/${playlistId}/tracks?limit=50&market=from_token`, { headers: { Authorization: `Bearer ${spotifyToken}` } });
    if (!resp.ok) { res.innerHTML = '<div class="emp"><h3>Ошибка загрузки</h3></div>'; return; }
    const data = await resp.json();
    const items = (data.items || []).filter(i => i?.track?.id);
    res.innerHTML = `
      <div style="display:flex;align-items:center;gap:14px;padding:12px 16px">
        ${img ? `<img src="${img}" style="width:52px;height:52px;border-radius:10px;object-fit:cover" alt="">` : ''}
        <div style="font-size:17px;font-weight:700">${esc(name)}</div>
      </div>`;
    items.forEach(({ track: item }) => {
      const art = item.album?.images?.[0]?.url || '';
      const artists = item.artists?.map(a => a.name).join(', ') || '';
      const dur = Math.round((item.duration_ms || 0) / 1000);
      const inLib = T.some(t => t.spotifyId === item.id);
      const el = document.createElement('div');
      el.className = 'ti';
      el.innerHTML = `
        <div class="ta">${art ? `<img src="${art}" alt="">` : '<span></span>'}</div>
        <div class="tin">
          <div class="tn">${esc(item.name)}${inLib ? ' <span style="color:#a78bfa;font-size:11px">✓</span>' : ''}</div>
          <div class="tar">${esc(artists)}</div>
        </div>
        <div class="rowact">
          <button class="tplus" title="Добавить" style="${inLib ? 'color:rgba(167,139,250,1)' : ''}">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
              <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
            </svg>
          </button>
          <div class="td">${fmt(dur)}</div>
        </div>`;
      el.querySelector('.tplus').addEventListener('click', async (ev) => {
        ev.stopPropagation();
        if (inLib) return;
        await spAddTrackToLibrary(item);
        ev.currentTarget.style.color = 'rgba(167,139,250,1)';
      });
      el.addEventListener('click', (e) => { if (e.target.closest('.tplus')) return; spPlayTrack(item); });
      res.appendChild(el);
    });
  } catch(e) {
    res.innerHTML = `<div class="emp"><h3>Ошибка</h3><p>${esc(String(e))}</p></div>`;
  }
}

// Открыть альбом из Spotify
async function spOpenAlbum(albumId, name, img) {
  if (!spotifyToken) return;
  const res = document.getElementById('sr');
  res.innerHTML = `<div class="s-loading">Загрузка альбома...</div>`;
  try {
    const resp = await fetch(`https://api.spotify.com/v1/albums/${albumId}/tracks?limit=50&market=from_token`, { headers: { Authorization: `Bearer ${spotifyToken}` } });
    if (!resp.ok) { res.innerHTML = '<div class="emp"><h3>Ошибка загрузки</h3></div>'; return; }
    const data = await resp.json();
    res.innerHTML = `
      <div style="display:flex;align-items:center;gap:14px;padding:12px 16px">
        ${img ? `<img src="${img}" style="width:52px;height:52px;border-radius:10px;object-fit:cover" alt="">` : ''}
        <div style="font-size:17px;font-weight:700">${esc(name)}</div>
      </div>`;
    (data.items || []).forEach(item => {
      const artists = item.artists?.map(a => a.name).join(', ') || '';
      const dur = Math.round((item.duration_ms || 0) / 1000);
      const el = document.createElement('div');
      el.className = 'ti';
      el.innerHTML = `
        <div class="ta"><span style="display:flex;align-items:center;justify-content:center;font-size:13px;color:rgba(255,255,255,0.4)">${item.track_number || ''}</span></div>
        <div class="tin">
          <div class="tn">${esc(item.name)}</div>
          <div class="tar">${esc(artists)}</div>
        </div>
        <div class="rowact"><div class="td">${fmt(dur)}</div></div>`;
      el.addEventListener('click', () => spPlayTrack({ ...item, album: { id: albumId, name, images: img ? [{ url: img }] : [] } }));
      res.appendChild(el);
    });
  } catch(e) {
    res.innerHTML = `<div class="emp"><h3>Ошибка</h3><p>${esc(String(e))}</p></div>`;
  }
}

async function scDoSearch(q) {
  const res = document.getElementById('sr');
  try {
    const clientId = await scGetClientId();
    res.innerHTML = '';
    let rendered = false;
    if (searchCategory === 'tracks') {
      const url = `https://api-v2.soundcloud.com/search/tracks?q=${encodeURIComponent(q)}&client_id=${clientId}&limit=20&linked_partitioning=1`;
      const text = await scFetch(url);
      if (!text) {
        res.innerHTML = '<div class="emp"><h3>Ошибка поиска</h3><p>Не удалось получить ответ от SoundCloud</p></div>';
        return;
      }
      const json = JSON.parse(text);
      const collection = Array.isArray(json.collection) ? json.collection : [];
      if (collection.length) {
        rendered = true;
        const ttit = document.createElement('div');
        ttit.className = 'sc-sec-tit';
        ttit.style.cssText = 'font-size: 17px; font-weight: 700; padding: 12px 16px; opacity: 0.8;';
        ttit.textContent = 'Треки';
        res.appendChild(ttit);
        collection.forEach(item => {
          const el = document.createElement('div');
          el.className = 'ti';
          const art = item.artwork_url ? item.artwork_url.replace('-large', '-t500x500') : (item.user ? item.user.avatar_url : '');
          const dur = (item.full_duration || item.duration || 0) / 1000;
          el.innerHTML = `
            <div class="ta">${art ? `<img src="${art}" alt="">` : '<span></span>'}</div>
            <div class="tin">
              <div class="tn">${esc(item.title)} <span class="sc-badge">SC</span></div>
              <div class="tar">${esc(item.user ? item.user.username : 'Unknown')}</div>
            </div>
            <div class="rowact"><div class="td">${fmt(dur)}</div></div>`;
          el.addEventListener('click', async () => {
            scToast('Загрузка...', item.title, true);
            try {
              const info = await scExtractTrackInfo(item, clientId);
              if (!info || !info.streamUrl) {
                const full = await scResolveTrack(item.permalink_url);
                if (full && full.streamUrl) return playSCResult(full);
                scToast('Ошибка', 'Не удалось получить поток', false);
                setTimeout(scToastHide, 3000);
                return;
              }
              await playSCResult(info);
            } catch (e) {
              scToast('Ошибка', String(e), false);
              setTimeout(scToastHide, 3000);
            }
          });
          res.appendChild(el);
        });
      }
    } else if (searchCategory === 'playlists') {
      rendered = await scDoSearchPlaylists(q, res);
    } else if (searchCategory === 'artists') {
      rendered = await scDoSearchArtists(q, res);
    }
    if (!rendered) {
      const labels = {
        tracks: 'треков',
        playlists: 'плейлистов',
        artists: 'исполнителей'
      };
      res.innerHTML = `<div class="emp"><h3>Ничего не найдено</h3><p>SoundCloud не нашёл ${labels[searchCategory] || 'ничего'} по этому запросу</p></div>`;
    }
    
  } catch (e) {
    res.innerHTML = `<div class="emp"><h3>Ошибка</h3><p>${esc(String(e))}</p></div>`;
  }
}

async function scDoSearchPlaylists(q, res) {
  try {
    const clientId = await scGetClientId();
    const url = `https://api-v2.soundcloud.com/search/playlists?q=${encodeURIComponent(q)}&client_id=${clientId}&limit=10`;
    const text = await scFetch(url);
    if (!text) return false;
    const json = JSON.parse(text);
    if (!json.collection || !json.collection.length) return false;
    
    const ttit = document.createElement('div');
    ttit.className = 'sc-sec-tit';
    ttit.style.cssText = 'font-size: 17px; font-weight: 700; padding: 16px 16px 8px; opacity: 0.8;';
    ttit.textContent = 'Плейлисты';
    res.appendChild(ttit);
    
    json.collection.forEach(pl => {
      const el = document.createElement('div');
      el.className = 'ti sc-playlist-item';
      const art = pl.artwork_url?.replace('-large', '-t300x300') || pl.tracks?.[0]?.artwork_url || '';
      el.innerHTML = `
        <div class="ta">${art ? `<img src="${art}">` : '<span>♪</span>'}</div>
        <div class="tin">
          <div class="tn">${esc(pl.title)} <span class="sc-badge sc-badge-pl" style="background:#f97316">PL</span></div>
          <div class="tar">${esc(pl.user?.username || '')} • ${pl.track_count} треков</div>
        </div>
        <button class="tplus" title="Импорт плейлиста">+</button>
      `;
      el.querySelector('.tplus').addEventListener('click', (e) => {
        e.stopPropagation();
        importSCPlaylist(pl);
      });
      el.addEventListener('click', () => previewSCPlaylist(pl));
      res.appendChild(el);
    });
    return true;
  } catch(e) {}
  return false;
}

async function scDoSearchArtists(q, res) {
  try {
    const clientId = await scGetClientId();
    const url = `https://api-v2.soundcloud.com/search/users?q=${encodeURIComponent(q)}&client_id=${clientId}&limit=8`;
    const text = await scFetch(url);
    if (!text) return false;
    const json = JSON.parse(text);
    if (!json.collection || !json.collection.length) return false;
    
    const ttit = document.createElement('div');
    ttit.className = 'sc-sec-tit';
    ttit.style.cssText = 'font-size: 17px; font-weight: 700; padding: 16px 16px 8px; opacity: 0.8;';
    ttit.textContent = 'Исполнители';
    res.appendChild(ttit);
    
    json.collection.forEach(user => {
      const el = document.createElement('div');
      el.className = 'ti sc-artist-item';
      el.innerHTML = `
        <div class="ta ta--round" style="border-radius:50%">
          ${user.avatar_url ? `<img src="${user.avatar_url.replace('-large','-t200x200')}" style="border-radius:50%">` : '<span style="border-radius:50%">👤</span>'}
        </div>
        <div class="tin">
          <div class="tn">${esc(user.username)} ${user.verified ? '<span style="color:#3b82f6;font-size:12px">✓</span>' : ''}</div>
          <div class="tar">Исполнитель ${user.followers_count ? '• ' + fmtNum(user.followers_count) + ' подписчиков' : ''}</div>
        </div>
      `;
      el.addEventListener('click', () => openArtistPage(user));
      res.appendChild(el);
    });
    return true;
  } catch(e) {}
  return false;
}

function fmtNum(n) {
  if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
  if (n >= 1000) return (n / 1000).toFixed(1) + 'K';
  return n.toString();
}

async function playSCResult(info) {
  scToastHide();
  const track = await scAddTrackFromInfo(info, info.permalinkUrl);
  if (track) {
    rl();
    renderPlaylists();
    saveSoundCloudCache();
    addTrackToSearchHistory(track);
    play(T.indexOf(track));
  }
}

async function importSCPlaylist(pl) {
  scToast('Импорт', 'Загрузка треков...', true);
  try {
    const clientId = await scGetClientId();
    const json = await scGetPlaylistJson(pl, clientId);
    const expandedTracks = await scExpandPlaylistTracks(json.tracks || [], clientId);
    if (!expandedTracks.length) {
      scToast('Ошибка', 'Плейлист пуст', false);
      setTimeout(scToastHide, 3000);
      return;
    }
    
    const localPl = createPlaylist(pl.title, `Импортировано из SoundCloud (${pl.user?.username || 'Unknown'})`);
    if (!localPl) return;
    
    let added = 0;
    for (const item of expandedTracks) {
      if (!item || !scTrackIdFromAny(item)) continue;
      const info = await scResolvePlaylistTrackInfo(item, clientId);
      if (info) {
        const track = await scAddTrackFromInfo(info, info.permalinkUrl, null, { deferArt: true });
        if (track) {
          ensureTrackId(track);
          localPl.trackIds.push(track.id);
          added++;
        }
      }
    }
    
    clearPreviewFlagsForPlaylistTracks(localPl);
    savePlaylistsCache();
    saveT();
    saveSoundCloudCache();
    renderPlaylists();
    rl();
    scToast('Успех', `Импортировано ${added} треков`, false);
    setTimeout(scToastHide, 3000);
  } catch (e) {
    scToast('Ошибка', 'Не удалось импортировать', false);
    setTimeout(scToastHide, 3000);
  }
}

async function previewSCPlaylist(pl) {
  scToast('Загрузка плейлиста...', pl.title, true);
  try {
    const clientId = await scGetClientId();
    const json = await scGetPlaylistJson(pl, clientId);
    const tracks = await scExpandPlaylistTracks(json.tracks || [], clientId);

    // Создаём временный плейлист только в памяти (не сохраняем)
    const tempId = 'sc_preview_' + pl.id;
    let localPl = P.find(x => x.id === tempId);
    const alreadyInLib = P.some(x => x.scSourceId === String(pl.id) && !x.id.startsWith('sc_preview_'));

    if (!localPl) {
      localPl = {
        id: tempId,
        name: pl.title,
        description: pl.user?.username || '',
        trackIds: [],
        source: 'soundcloud',
        scSourceId: String(pl.id),
        _preview: true
      };
      P.push(localPl);
    }
    localPl.trackIds = [];

    const artHydrationQueue = [];
    for (const item of tracks) {
      if (!item || !scTrackIdFromAny(item)) continue;
      const sourceUrl = (item.permalink_url && String(item.permalink_url)) || '';
      const track = scBuildLightTrackFromJson(item, sourceUrl);
      if (!track) continue;
      ensureTrackId(track);
      if (!localPl.trackIds.includes(track.id)) localPl.trackIds.push(track.id);
      artHydrationQueue.push({ track, artworkUrl: scGetArtworkUrl(item) });
    }

    scToastHide();
    openPlaylistView(tempId, true);
    void scHydratePlaylistArtworkInBackground(artHydrationQueue);

    // Настраиваем кнопку плюс — добавить в медиатеку
    const plusBtn = document.getElementById('vpl-act-like');
    if (plusBtn) {
      if (alreadyInLib) {
        plusBtn.style.color = 'rgba(167,139,250,1)';
        plusBtn.title = 'Уже в медиатеке';
        plusBtn.onclick = null;
      } else {
        plusBtn.style.color = 'rgba(255,255,255,0.5)';
        plusBtn.title = 'Добавить в медиатеку';
        plusBtn.onclick = () => {
          // Конвертируем preview в постоянный плейлист
          const idx = P.findIndex(x => x.id === tempId);
          if (idx >= 0) {
            P[idx].id = 'pl_' + Date.now().toString(36);
            P[idx]._preview = false;
            clearPreviewFlagsForPlaylistTracks(P[idx]);
            savePlaylistsCache();
            saveT();
            saveSoundCloudCache();
            renderPlaylists();
            if (typeof window.sbEnsurePlaylistUploaded === 'function') {
              window.sbEnsurePlaylistUploaded(P[idx], { immediate: true, hydrateTracks: true }).catch((e) => {
                console.warn('[SB-SYNC] Immediate preview playlist upload failed:', e);
              });
            }
            plusBtn.style.color = 'rgba(167,139,250,1)';
            plusBtn.title = 'В медиатеке';
            plusBtn.onclick = null;
            scToast('Добавлено', pl.title, false);
            setTimeout(scToastHide, 2000);
          }
        };
      }
    }
  } catch(e) {
    scToast('Ошибка', String(e).substring(0, 80), false);
    setTimeout(scToastHide, 3000);
  }
}

//  KEYBOARD 
document.addEventListener('keydown', e => {
  if (e.target.tagName === 'INPUT') return;
  if (e.key === ' ') { e.preventDefault(); togglePlay(); }
  else if (e.key === 'ArrowRight') playNext();
  else if (e.key === 'ArrowLeft') playPrev();
});

//  UTILS 
function fmt(s) { if (!s || isNaN(s)) return '0:00'; return `${Math.floor(s / 60)}:${Math.floor(s % 60).toString().padStart(2, '0')}`; }
function esc(s) { return s.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c])); }
function pl(n) { if (n % 10 === 1 && n % 100 !== 11) return 'трек'; if ([2, 3, 4].includes(n % 10) && ![12, 13, 14].includes(n % 100)) return 'трека'; return 'треков'; }

// ── LAST TRACK PERSISTENCE ──
function saveLastTrack(t) {
  if (!t) return;
  ensureTrackId(t);
  try {
    const data = { id: t.id, title: t.title, artist: t.artist, art: null, cur: cur };
    // Don't save heavy art data, just the reference
    saveAppStateValue(LAST_TRACK_KEY, JSON.stringify(data));
  } catch (_) { }
}

async function restoreLastTrack() {
  try {
    const raw = await loadAppStateValue(LAST_TRACK_KEY);
    if (!raw) {
      console.log('[restore] no saved track found');
      return;
    }
    const data = JSON.parse(raw);
    if (!data || !data.id) {
      console.log('[restore] invalid saved track data');
      return;
    }
    
    // Find the track in the current list
    let idx = T.findIndex(t => t.id === data.id);
    
    // Fallback: try to find by title and artist if ID fails (useful for local files that lost their ID)
    if (idx < 0) {
      console.log('[restore] ID mismatch, trying fallback match for', data.title);
      idx = T.findIndex(t => t.title === data.title && t.artist === data.artist);
    }

    if (idx < 0) {
      console.log('[restore] track not found in list. ID:', data.id, 'Title:', data.title);
      return;
    }
    
    const t = T[idx];
    cur = idx;
    console.log('[restore] restoring track:', t.title);
    
    // Don't auto-play, just set up the UI
    try {
      if (t.url) setAudSrcIfChanged(t.url);
      upUI(t);
      upMini(t);
      extractColor(t, playEpoch);
      upBtns();
      const mini = document.getElementById('mini');
      const onPlayerView = document.getElementById('vp').classList.contains('active');
      mini.classList.toggle('show', !onPlayerView);
      syncMediaSessionTrack(t);

      // Scroll to the track in the list after a short delay to ensure DOM is ready
      setTimeout(() => {
        const list = document.getElementById('tl');
        const activeEl = list ? list.querySelector('.ti.now') : null;
        if (activeEl && list) {
          list.scrollTo({
            top: activeEl.offsetTop - list.offsetTop - (list.clientHeight / 2) + (activeEl.clientHeight / 2),
            behavior: 'smooth'
          });
        }
      }, 100);

    } catch (e) { console.warn('[restore] UI update failed', e); }
  } catch (e) {
    console.warn('[restore] failed', e);
  }
}

// ── NEIGHBOR COVER PRELOADING ──
function preloadNeighborCovers() {
  if (!T.length || cur < 0) return;
  const len = T.length;
  const prevIdx = (cur - 1 + len) % len;
  const nextIdx = (cur + 1) % len;
  [prevIdx, nextIdx].forEach(idx => {
    const t = T[idx];
    if (!t) return;
    ensureTrackId(t);
    if (_preloadedArtCache.has(t.id)) return;

    if (t.art) {
      // Art already loaded — just preload the image
      const img = new Image();
      img.src = t.art;
      _preloadedArtCache.set(t.id, img);
    } else if (t.isSoundCloud && t.scTrackId) {
      // SC track without art — pre-fetch art in background
      _preloadedArtCache.set(t.id, true); // Mark as in-progress
      (async () => {
        try {
          const clientId = await scGetClientId();
          const info = await scResolveTrackById(String(t.scTrackId), clientId);
          if (info && info.artworkUrl) {
            const art = await scFetchArt(info.artworkUrl);
            if (art) {
              t.art = art;
              const img = new Image();
              img.src = art;
              _preloadedArtCache.set(t.id, img);
            }
          }
        } catch (_) { }
      })();
    }
  });
  // Keep cache small
  if (_preloadedArtCache.size > 20) {
    const keys = [..._preloadedArtCache.keys()];
    for (let i = 0; i < keys.length - 20; i++) _preloadedArtCache.delete(keys[i]);
  }
}

// ── PRE-FETCH NEXT TRACK STREAM ──
async function prefetchNextTrackStream() {
  if (_prefetchingNextTrack || !T.length || cur < 0) return;
  const nextIdx = shuf ? Math.floor(Math.random() * T.length) : (cur + 1) % T.length;
  const nextT = T[nextIdx];
  if (!nextT) return;
  ensureTrackId(nextT);
  // Skip if already prefetched or is a local file
  if (_prefetchedStreamCache.has(nextT.id)) return;
  if (nextT.file && nextT.url && nextT.url.startsWith('blob:')) return;
  // Only prefetch SoundCloud/Spotify tracks that need stream resolution
  if (!nextT.isSoundCloud && !nextT.isSpotify) return;
  _prefetchingNextTrack = true;
  try {
    if (nextT.isSoundCloud && nextT.scTrackId) {
      const clientId = await scGetClientId();
      const info = await scResolveTrackById(String(nextT.scTrackId), clientId);
      if (info && info.streamUrl) {
        const port = await scGetProxyPort();
        nextT.streamUrl = info.streamUrl;
        nextT.url = `http://127.0.0.1:${port}/stream?url=${encodeURIComponent(info.streamUrl)}`;
        _prefetchedStreamCache.set(nextT.id, info.streamUrl);
        console.log('[prefetch] Pre-fetched stream for next track:', nextT.title);
      }
    } else if (nextT.isSpotify && (!nextT.url || !nextT.streamUrl)) {
      // Search SoundCloud for Spotify track
      const q = `${nextT.artist} - ${nextT.title}`;
      const clientId = await scGetClientId();
      const scSearchUrl = `https://api-v2.soundcloud.com/search/tracks?q=${encodeURIComponent(q)}&client_id=${clientId}&limit=3`;
      const text = await scFetch(scSearchUrl);
      if (text) {
        const json = JSON.parse(text);
        const results = json.collection || [];
        const match = results.find(r =>
          r.title.toLowerCase().includes(nextT.title.toLowerCase()) ||
          nextT.title.toLowerCase().includes(r.title.toLowerCase())
        ) || results[0];
        if (match) {
          const info = await scExtractTrackInfo(match, clientId);
          if (info && info.streamUrl) {
            const port = await scGetProxyPort();
            nextT.streamUrl = info.streamUrl;
            nextT.url = `http://127.0.0.1:${port}/stream?url=${encodeURIComponent(info.streamUrl)}`;
            nextT.isSoundCloud = true;
            nextT.isSpotify = false;
            _prefetchedStreamCache.set(nextT.id, info.streamUrl);
            console.log('[prefetch] Pre-fetched Spotify->SC stream for next track:', nextT.title);
          }
        }
      }
    }
  } catch (e) {
    console.warn('[prefetch] Failed to pre-fetch next track stream:', e);
  } finally {
    _prefetchingNextTrack = false;
  }
}

// ── QUEUE OVERLAY ──
function openQueue() {
  const ov = document.getElementById('queue-ov');
  if (!ov) return;
  ov.classList.remove('closing');
  renderQueueList();
  ov.classList.add('show');
}

function closeQueue() {
  closeOv('queue-ov');
}

function closeOv(id) {
  const ov = document.getElementById(id);
  if (!ov) return;
  ov.classList.add('closing');
  setTimeout(() => {
    ov.classList.remove('show');
    ov.classList.remove('closing');
  }, 450);
}

function closeTrackPlaylistModal() {
  closeOv('trk-pl-ov');
}

function closePlaylistCreate() {
  closeOv('pl-create-ov');
}

function renderQueueList() {
  const list = document.getElementById('queue-list');
  if (!list) return;
  list.innerHTML = '';
  if (!T.length || cur < 0) {
    list.innerHTML = '<div class="emp"><p>Очередь пуста</p></div>';
    return;
  }
  // Show upcoming tracks starting from current
  const len = T.length;
  const maxShow = Math.min(len, 50);
  for (let offset = 0; offset < maxShow; offset++) {
    const idx = (cur + offset) % len;
    const t = T[idx];
    if (!t) continue;
    const el = document.createElement('div');
    el.className = 'ti' + (idx === cur ? ' now' : '');
    const badge = offset === 0 ? '<span class="sc-badge" style="background:rgba(167,139,250,0.3);color:#a78bfa">▶</span>' : '';
    el.innerHTML = `
      <div class="ta">${t.art ? `<img src="${t.art}" alt="">` : '<span></span>'}</div>
      <div class="tin">
        <div class="tn">${badge} ${esc(t.title)}</div>
        <div class="tar">${esc(t.artist)}</div>
      </div>
      <div class="rowact"><div class="td">${fmt(t.duration)}</div></div>`;
    el.addEventListener('click', () => {
      play(idx);
      closeQueue();
    });
    list.appendChild(el);
  }
}

// ── OFFLINE MODE ──
renderQueueList = function() {
  const list = document.getElementById('queue-list');
  if (!list) return;
  list.innerHTML = '';
  const entries = getPlaybackEntries();
  if (!entries.length || cur < 0) {
    list.innerHTML = '<div class="emp"><p>ÐžÑ‡ÐµÑ€ÐµÐ´ÑŒ Ð¿ÑƒÑÑ‚Ð°</p></div>';
    return;
  }
  const currentPos = getPlaybackPosition(entries, cur);
  const startPos = currentPos >= 0 ? currentPos : 0;
  const maxShow = Math.min(entries.length, 50);
  for (let offset = 0; offset < maxShow; offset++) {
    const entry = entries[(startPos + offset) % entries.length];
    const idx = entry ? entry.idx : -1;
    const t = entry ? entry.track : null;
    if (!t) continue;
    const el = document.createElement('div');
    el.className = 'ti' + (idx === cur ? ' now' : '');
    const badge = offset === 0 ? '<span class="sc-badge" style="background:rgba(167,139,250,0.3);color:#a78bfa">â–¶</span>' : '';
    el.innerHTML = `
      <div class="ta">${t.art ? `<img src="${t.art}" alt="">` : '<span></span>'}</div>
      <div class="tin">
        <div class="tn">${badge} ${esc(t.title)}</div>
        <div class="tar">${esc(t.artist)}</div>
      </div>
      <div class="rowact"><div class="td">${fmt(t.duration)}</div></div>`;
    el.addEventListener('click', () => {
      play(idx);
      closeQueue();
    });
    list.appendChild(el);
  }
}

renderQueueList = function() {
  const list = document.getElementById('queue-list');
  if (!list) return;
  list.innerHTML = '';
  const entries = getPlaybackEntries();
  if (!entries.length || cur < 0) {
    list.innerHTML = '<div class="emp"><p>\u041E\u0447\u0435\u0440\u0435\u0434\u044C \u043F\u0443\u0441\u0442\u0430</p></div>';
    return;
  }
  const currentPos = getPlaybackPosition(entries, cur);
  const startPos = currentPos >= 0 ? currentPos : 0;
  const maxShow = Math.min(entries.length, 50);
  for (let offset = 0; offset < maxShow; offset++) {
    const entry = entries[(startPos + offset) % entries.length];
    const idx = entry ? entry.idx : -1;
    const t = entry ? entry.track : null;
    if (!t) continue;
    const el = document.createElement('div');
    el.className = 'ti' + (idx === cur ? ' now' : '');
    const badge = offset === 0 ? '<span class="sc-badge" style="background:rgba(167,139,250,0.3);color:#a78bfa">&#9654;</span>' : '';
    el.innerHTML = `
      <div class="ta">${t.art ? `<img src="${t.art}" alt="">` : '<span></span>'}</div>
      <div class="tin">
        <div class="tn">${badge} ${esc(t.title)}</div>
        <div class="tar">${esc(t.artist)}</div>
      </div>
      <div class="rowact"><div class="td">${fmt(t.duration)}</div></div>`;
    el.addEventListener('click', () => {
      play(idx);
      closeQueue();
    });
    list.appendChild(el);
  }
}

function toggleOfflineMode() {
  const toggle = document.getElementById('offline-toggle');
  offlineModeEnabled = toggle ? toggle.checked : !offlineModeEnabled;
  localStorage.setItem(OFFLINE_MODE_KEY, JSON.stringify(offlineModeEnabled));
  rl();
}

async function initOfflineMode() {
  offlineModeEnabled = JSON.parse(localStorage.getItem(OFFLINE_MODE_KEY) || 'false');
  const toggle = document.getElementById('offline-toggle');
  if (toggle) toggle.checked = offlineModeEnabled;
  // Mark tracks that are cached offline
  await markOfflineCachedTracks();
}

async function markOfflineCachedTracks() {
  try {
    const db = await openOfflineAudioDb();
    if (!db) return;
    const tx = db.transaction(OFFLINE_AUDIO_STORE, 'readonly');
    const store = tx.objectStore(OFFLINE_AUDIO_STORE);
    const req = store.getAllKeys();
    req.onsuccess = () => {
      const keys = new Set(req.result || []);
      T.forEach(t => {
        if (t.id && keys.has(t.id)) {
          t._offlineCached = true;
        }
      });
      rl(); // Refresh UI to show badges
    };
  } catch (e) {
    console.warn('[offline] Failed to mark cached tracks', e);
  }
}

async function downloadTrackForOffline(track) {
  if (!track) return false;
  ensureTrackId(track);
  if (!track.url || !isNetworkAudioUrl(track.url)) {
    // May need to resolve stream first
    if (track.isSoundCloud && track.scTrackId) {
      try {
        const clientId = await scGetClientId();
        const info = await scResolveTrackById(String(track.scTrackId), clientId);
        if (info && info.streamUrl) {
          const port = await scGetProxyPort();
          track.streamUrl = info.streamUrl;
          track.url = `http://127.0.0.1:${port}/stream?url=${encodeURIComponent(info.streamUrl)}`;
        }
      } catch (e) {
        console.warn('[offline] Failed to resolve stream for download', e);
        return false;
      }
    }
    if (!track.url) return false;
  }
  try {
    await cacheTrackAudioForOffline(track, track.url);
    track._offlineCached = true;
    rl();
    return true;
  } catch (e) {
    console.warn('[offline] Download failed', e);
    return false;
  }
}

// Init
setA(167, 139, 250);
renderPlaylists();

//  SOUNDCLOUD INTEGRATION 
let scClientId = null;
let scProxyPort = 19867; // совпадает с PROXY_PORT в lib.rs
const SC_CACHE_KEY = 'sc_tracks_cache_v1';
const SC_ART_CACHE_KEY = 'sc_art_cache_v1';

async function scGetProxyPort() {
  try {
    const inv = getTauriInvoke();
    if (typeof inv === 'function') {
      scProxyPort = await inv('get_proxy_port');
    }
  } catch (e) { }
  return scProxyPort;
}

// Вызови один раз при старте:
scGetProxyPort();

function saveSoundCloudCache() {
  try {
    const artMap = {};
    const data = T
      .filter(t => t && t.isSoundCloud && (t.streamUrl || isTrackReferencedBySavedPlaylist(ensureTrackId(t))))
      .map(t => ({
        scTrackId: t.scTrackId || '',
        sourceUrl: t.sourceUrl || t.path || '',
        streamUrl: t.streamUrl || '',
        url: t.url || '',
        title: t.title || 'Unknown Title',
        artist: t.artist || 'Unknown Artist',
        album: t.album || 'SoundCloud',
        duration: Number(t.duration) || 0,
        art: t.art || null,
        pendingArtworkUrl: t._pendingArtworkUrl || '',
        id: t.id || '',
        previewTransient: !!t._previewTransient
      }));
    data.forEach(item => {
      if (item.scTrackId && item.art) artMap[item.scTrackId] = item.art;
    });
    localStorage.setItem(SC_CACHE_KEY, JSON.stringify(data));
    localStorage.setItem(SC_ART_CACHE_KEY, JSON.stringify(artMap));
  } catch (e) { }
}

async function hydrateSoundCloudCache() {
  let parsed = null;
  try {
    const raw = localStorage.getItem(SC_CACHE_KEY);
    if (!raw) return;
    parsed = JSON.parse(raw);
  } catch (e) { return; }
  if (!Array.isArray(parsed) || !parsed.length) return;
  let artCache = {};
  try {
    const rawArt = localStorage.getItem(SC_ART_CACHE_KEY);
    artCache = rawArt ? JSON.parse(rawArt) || {} : {};
  } catch (e) { }
  const port = await scGetProxyPort();
  parsed.forEach(item => {
    if (!item) return;
    const scTrackId = String(item.scTrackId || '');
    const proxiedUrl = item.streamUrl ? `http://127.0.0.1:${port}/stream?url=${encodeURIComponent(item.streamUrl)}` : (item.url || '');
    if (scTrackId && T.some(x => x.scTrackId === scTrackId)) return;
    if (!scTrackId && proxiedUrl && T.some(x => x.url === proxiedUrl)) return;
    const art = item.art || ((scTrackId && artCache[scTrackId]) ? artCache[scTrackId] : null);
    const track = {
      file: null,
      id: item.id || (scTrackId ? `sc_${scTrackId}` : undefined),
      scTrackId: scTrackId || '',
      path: item.sourceUrl || `soundcloud:${item.title || 'track'}`,
      sourceUrl: item.sourceUrl || '',
      streamUrl: item.streamUrl || '',
      url: proxiedUrl || '',
      title: item.title || 'Unknown Title',
      artist: item.artist || 'Unknown Artist',
      album: item.album || 'SoundCloud',
      duration: Number(item.duration) || 0,
      art,
      liked: false,
      isSoundCloud: true
    };
    if (item.pendingArtworkUrl) track._pendingArtworkUrl = item.pendingArtworkUrl;
    if (item.previewTransient && !isTrackReferencedBySavedPlaylist(track.id || `sc_${scTrackId}`)) {
      track._previewTransient = true;
    }
    ensureTrackId(track);
    T.push(track);
  });
  rl();
}

function getTauriInvoke() {
  if (window.__TAURI__) {
    if (window.__TAURI__.core && window.__TAURI__.core.invoke) return window.__TAURI__.core.invoke;
    if (window.__TAURI__.invoke) return window.__TAURI__.invoke;
  }
  if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) return window.__TAURI_INTERNALS__.invoke;
  return null;
}

const pendingFrontendRustLogs = [];

function queueFrontendRustLog(kind, a, b) {
  pendingFrontendRustLogs.push([kind, a, b]);
  if (pendingFrontendRustLogs.length > 200) pendingFrontendRustLogs.shift();
}

function flushFrontendRustLogs() {
  if (!pendingFrontendRustLogs.length) return;
  const inv = getTauriInvoke();
  if (!inv) return;
  while (pendingFrontendRustLogs.length) {
    const [kind, a, b] = pendingFrontendRustLogs.shift();
    try {
      if (kind === 'scope') {
        inv('log_frontend_debug', { scope: String(a || 'ui'), message: String(b || '') }).catch(() => { });
      } else {
        inv('log_frontend_console', { level: String(a || 'info'), message: String(b || '') }).catch(() => { });
      }
    } catch (_) { }
  }
}

function logRustConsole(scope, message) {
  const inv = getTauriInvoke();
  if (!inv) {
    queueFrontendRustLog('scope', scope, message);
    return;
  }
  try {
    inv('log_frontend_debug', { scope: String(scope || 'ui'), message: String(message || '') }).catch(() => { });
  } catch (_) { }
}

function logRustConsoleLevel(level, message) {
  const inv = getTauriInvoke();
  if (!inv) {
    queueFrontendRustLog('level', level, message);
    return;
  }
  try {
    inv('log_frontend_console', {
      level: String(level || 'info'),
      message: String(message || '')
    }).catch(() => { });
  } catch (_) { }
}

function formatConsoleArg(arg, seen) {
  if (arg instanceof Error) {
    const stack = arg.stack ? `\n${arg.stack}` : '';
    return `${arg.name || 'Error'}: ${arg.message || String(arg)}${stack}`;
  }
  if (typeof arg === 'string') return arg;
  if (typeof arg === 'number' || typeof arg === 'boolean' || arg == null) return String(arg);
  if (typeof arg === 'bigint') return `${arg}n`;
  if (typeof arg === 'function') return `[Function ${arg.name || 'anonymous'}]`;
  if (!seen) seen = new WeakSet();
  try {
    return JSON.stringify(arg, (key, value) => {
      if (value instanceof Error) {
        return {
          name: value.name,
          message: value.message,
          stack: value.stack || null
        };
      }
      if (typeof value === 'bigint') return `${value}n`;
      if (typeof value === 'object' && value !== null) {
        if (seen.has(value)) return '[Circular]';
        seen.add(value);
      }
      return value;
    });
  } catch (_) {
    try { return String(arg); } catch (_) { return '[Unserializable]'; }
  }
}

function describeInvokeError(err) {
  if (!err) return 'Unknown error';
  if (err instanceof Error) {
    return `${err.name || 'Error'}: ${err.message || String(err)}${err.stack ? `\n${err.stack}` : ''}`;
  }
  if (typeof err === 'string') return err;
  const parts = [];
  try {
    if (typeof err.message === 'string' && err.message) parts.push(`message=${err.message}`);
    if (typeof err.error === 'string' && err.error) parts.push(`error=${err.error}`);
    if (typeof err.code !== 'undefined') parts.push(`code=${err.code}`);
    if (typeof err.name === 'string' && err.name) parts.push(`name=${err.name}`);
    if (typeof err.stack === 'string' && err.stack) parts.push(`stack=${err.stack}`);
  } catch (_) { }
  const formatted = formatConsoleArg(err);
  if (parts.length && formatted && formatted !== '[Unserializable]') {
    return `${parts.join(' ')} raw=${formatted}`;
  }
  return formatted || String(err);
}

function setupRustConsoleBridge() {
  if (window.__liquifyRustConsoleBridgeInstalled) return;
  window.__liquifyRustConsoleBridgeInstalled = true;
  flushFrontendRustLogs();

  const original = {
    log: console.log.bind(console),
    info: console.info.bind(console),
    warn: console.warn.bind(console),
    error: console.error.bind(console),
    debug: (console.debug || console.log).bind(console)
  };

  const forward = (level, args) => {
    try {
      const text = Array.from(args || []).map((arg) => formatConsoleArg(arg)).join(' ');
      if (!text) return;
      logRustConsoleLevel(level, text.slice(0, 7000));
    } catch (_) { }
  };

  console.log = (...args) => {
    original.log(...args);
    forward('info', args);
  };
  console.info = (...args) => {
    original.info(...args);
    forward('info', args);
  };
  console.warn = (...args) => {
    original.warn(...args);
    forward('warn', args);
  };
  console.error = (...args) => {
    original.error(...args);
    forward('error', args);
  };
  console.debug = (...args) => {
    original.debug(...args);
    forward('debug', args);
  };

  window.addEventListener('error', (event) => {
    const msg = event?.message || 'unknown error';
    const src = event?.filename || 'unknown source';
    const line = event?.lineno || 0;
    const col = event?.colno || 0;
    const err = event?.error ? formatConsoleArg(event.error) : '';
    logRustConsoleLevel('error', `window.onerror ${msg} at ${src}:${line}:${col}${err ? ` ${err}` : ''}`);
  });

  window.addEventListener('unhandledrejection', (event) => {
    logRustConsoleLevel('error', `unhandledrejection ${formatConsoleArg(event?.reason)}`);
  });

  logRustConsoleLevel('info', 'rust console bridge installed');
  flushFrontendRustLogs();
}

function isAndroidRuntime() {
  try {
    return /Android/i.test(navigator.userAgent || '');
  } catch (_) {
    return false;
  }
}

function canvasConsole(message, details) {
  const text = `CANVAS ${String(message || '')}`;
  const payload = typeof details === 'undefined' ? text : `${text} ${formatConsoleArg(details)}`;
  pushCanvasOverlayLog('info', payload);
  logRustConsole('canvas', payload);
  logRustConsoleLevel('info', payload);
  if (typeof details === 'undefined') {
    console.log(text);
    return;
  }
  console.log(text, details);
}

function canvasConsoleWarn(message, details) {
  const text = `CANVAS ${String(message || '')}`;
  const payload = typeof details === 'undefined' ? text : `${text} ${formatConsoleArg(details)}`;
  pushCanvasOverlayLog('warn', payload);
  logRustConsole('canvas', payload);
  logRustConsoleLevel('warn', payload);
  if (typeof details === 'undefined') {
    console.warn(text);
    return;
  }
  console.warn(text, details);
}

function canvasConsoleError(message, details) {
  const text = `CANVAS ${String(message || '')}`;
  const payload = typeof details === 'undefined' ? text : `${text} ${formatConsoleArg(details)}`;
  pushCanvasOverlayLog('error', payload);
  logRustConsole('canvas', payload);
  logRustConsoleLevel('error', payload);
  if (typeof details === 'undefined') {
    console.error(text);
    return;
  }
  console.error(text, details);
}

function clearPlayerCanvasLayer() {
  const vp = document.getElementById('vp');
  const video = document.getElementById('pcv-video');
  activeCanvasInfo = null;
  if (vp) vp.classList.remove('has-canvas');
  logRustConsole('canvas', 'clearPlayerCanvasLayer');
  canvasConsole('clearPlayerCanvasLayer');
  if (!video) return;
  try { video.pause(); } catch (_) { }
  delete video.dataset.canvasFallbackTried;
  delete video.dataset.canvasPendingTrackId;
  video.removeAttribute('src');
  video.load();
}

function applyPlayerCanvasLayer(info) {
  const vp = document.getElementById('vp');
  const video = document.getElementById('pcv-video');
  if (!vp || !video) {
    canvasConsoleWarn('applyPlayerCanvasLayer aborted missing elements', { hasVp: !!vp, hasVideo: !!video });
    return;
  }
  const src = info?.canvasUrl || info?.proxiedCanvasUrl;
  logRustConsole('canvas', `applyPlayerCanvasLayer type=${info?.canvasType || 'none'} src=${src ? src.slice(0, 140) : 'null'}`);
  canvasConsole('applyPlayerCanvasLayer', {
    trackId: info?.trackId || null,
    canvasType: info?.canvasType || null,
    src: src ? src.slice(0, 200) : null
  });
  if (!src || info?.canvasType !== 'video') {
    canvasConsoleWarn('applyPlayerCanvasLayer skipped non-video canvas', {
      trackId: info?.trackId || null,
      canvasType: info?.canvasType || null,
      hasSrc: !!src
    });
    clearPlayerCanvasLayer();
    return;
  }
  activeCanvasInfo = info;
  delete video.dataset.canvasFallbackTried;
  video.dataset.canvasPendingTrackId = info?.trackId || '';
  video.crossOrigin = 'anonymous';
  if (video.src !== src) video.src = src;
  canvasConsole('video src assigned', { trackId: info?.trackId || null, src: (video.currentSrc || video.src || '').slice(0, 200) });
  const playPromise = video.play();
  if (playPromise && typeof playPromise.catch === 'function') {
    playPromise.catch((err) => {
      canvasConsoleWarn('video.play failed in applyPlayerCanvasLayer', { trackId: info?.trackId || null, error: err?.message || String(err) });
      logRustConsole('canvas', `video.play failed ${err?.message || String(err)}`);
      showCanvasToast('Canvas Spotify', `Ошибка показа: ${err?.message || String(err)}`, false, 3500);
    });
  }
}

async function applyPlayerCanvasLayerCached(info) {
  const vp = document.getElementById('vp');
  const video = document.getElementById('pcv-video');
  if (!vp || !video) {
    canvasConsoleWarn('applyPlayerCanvasLayerCached aborted missing elements', { hasVp: !!vp, hasVideo: !!video });
    return;
  }
  
  const src = info?.canvasUrl;
  if (!src || info?.canvasType !== 'video') {
    canvasConsoleWarn('applyPlayerCanvasLayerCached skipped non-video canvas', {
      trackId: info?.trackId || null,
      canvasType: info?.canvasType || null,
      hasSrc: !!src
    });
    clearPlayerCanvasLayer();
    return;
  }
  
  const inv = getTauriInvoke();
  if (!inv) {
    canvasConsoleWarn('cache_canvas_video invoke unavailable, falling back to direct src', { trackId: info?.trackId || null });
    applyPlayerCanvasLayer(info);
    return;
  }
  
  logRustConsole('canvas', `caching video for track=${info.trackId}`);
  canvasConsole('cache_canvas_video start', { trackId: info?.trackId || null, canvasUrl: src.slice(0, 200) });
  showCanvasToast('Canvas Spotify', `Ссылка получена: ${info.trackId || 'track'}`, true);
  
  try {
    const localPath = await inv('cache_canvas_video', {
      trackId: info.trackId || '',
      canvasUrl: src
    });
    canvasConsole('cache_canvas_video result', { trackId: info?.trackId || null, localPath: localPath || null });
    
    if (!localPath) {
      canvasConsoleWarn('cache_canvas_video returned empty path', { trackId: info?.trackId || null });
      clearPlayerCanvasLayer();
      return;
    }
    
    if (cur < 0 || !T[cur]?.spotifyId || 
        (info.trackId && T[cur].spotifyId !== info.trackId)) {
      logRustConsole('canvas', 'track changed during cache download, skip');
      canvasConsoleWarn('track changed during cache download', {
        requestedTrackId: info?.trackId || null,
        currentTrackId: T[cur]?.spotifyId || null,
        cur
      });
      return;
    }
    
    const convert = getTauriConvertFileSrc();
    const localPlayableUrl = typeof convert === 'function'
      ? convert(localPath)
      : `file://${localPath}`;
    const playableUrl = (isAndroidRuntime() && info?.proxiedCanvasUrl)
      ? info.proxiedCanvasUrl
      : localPlayableUrl;
    
    logRustConsole('canvas', `applying cached canvas from ${playableUrl.slice(0, 80)}`);
    canvasConsole('cached canvas ready', {
      trackId: info?.trackId || null,
      playableUrl: playableUrl.slice(0, 200),
      localPlayableUrl: localPlayableUrl.slice(0, 200),
      usingAndroidProxy: !!(isAndroidRuntime() && info?.proxiedCanvasUrl)
    });
    showCanvasToast('Canvas Spotify', 'Канвас загружен', true);
    
    activeCanvasInfo = info;
    video.dataset.canvasPendingTrackId = info?.trackId || '';
    video.crossOrigin = 'anonymous';
    
    if (video.src !== playableUrl) {
      video.src = playableUrl;
    }
    canvasConsole('video src assigned from cache', { trackId: info?.trackId || null, src: (video.currentSrc || video.src || '').slice(0, 200) });
    
    const playPromise = video.play();
    if (playPromise?.catch) {
      playPromise.catch(err => {
        const currentSrc = video.currentSrc || video.src || '';
        if (video.dataset.canvasFallbackTried === '1' || currentSrc !== playableUrl) {
          canvasConsoleWarn('video.play rejection ignored because canvas source already changed', {
            trackId: info?.trackId || null,
            error: err?.message || String(err),
            currentSrc: currentSrc.slice(0, 200),
            expectedSrc: playableUrl.slice(0, 200)
          });
          return;
        }
        logRustConsole('canvas', `play failed: ${err?.message || err}`);
        canvasConsoleError('video.play failed for cached canvas', { trackId: info?.trackId || null, error: err?.message || String(err) });
        showCanvasToast('Canvas Spotify', `Ошибка показа: ${err?.message || err}`, false, 3500);
        clearPlayerCanvasLayer();
      });
    }
    
  } catch (e) {
    logRustConsole('canvas', `cache failed: ${e?.message || String(e)}`);
    canvasConsoleError('cache_canvas_video failed', { trackId: info?.trackId || null, error: e?.message || String(e) });
    showCanvasToast('Canvas Spotify', `Ошибка загрузки: ${e?.message || String(e)}`, false, 3500);
    applyPlayerCanvasLayer(info);
  }
}

async function refreshPlayerCanvas(track) {
  const reqId = ++playerCanvasRequestId;
  logRustConsole('canvas', `refresh start req=${reqId} title=${track?.title || 'none'} spotifyId=${track?.spotifyId || 'none'}`);
  canvasConsole('refresh start', {
    reqId,
    title: track?.title || null,
    spotifyId: track?.spotifyId || null,
    cur
  });
  clearPlayerCanvasLayer();
  if (!track || !track.spotifyId) {
    canvasConsoleWarn('refresh skipped because track has no spotifyId', {
      title: track?.title || null,
      spotifyId: track?.spotifyId || null
    });
    return;
  }

  const cacheKey = String(track.spotifyId);
  let info = spotifyCanvasCache.get(cacheKey) || null;
  if (!info && track.spotifyCanvasUrl && track.spotifyCanvasType) {
    logRustConsole('canvas', `cache hit from track object spotifyId=${cacheKey} type=${track.spotifyCanvasType}`);
    canvasConsole('track object canvas cache hit', {
      spotifyId: cacheKey,
      canvasType: track.spotifyCanvasType,
      canvasUrl: (track.spotifyCanvasUrl || '').slice(0, 200),
      proxiedCanvasUrl: (track.spotifyCanvasProxyUrl || '').slice(0, 200)
    });
    info = {
      canvasUrl: track.spotifyCanvasUrl,
      proxiedCanvasUrl: track.spotifyCanvasProxyUrl || null,
      canvasType: track.spotifyCanvasType,
      trackId: track.spotifyId
    };
    spotifyCanvasCache.set(cacheKey, info);
  }
  if (info) {
    logRustConsole('canvas', `memory cache hit spotifyId=${cacheKey} type=${info.canvasType || 'unknown'}`);
    canvasConsole('memory canvas cache hit', {
      spotifyId: cacheKey,
      canvasType: info.canvasType || null,
      canvasUrl: (info.canvasUrl || '').slice(0, 200),
      proxiedCanvasUrl: (info.proxiedCanvasUrl || '').slice(0, 200)
    });
  }

  if (!info) {
    const inv = getTauriInvoke();
    if (!inv) {
      logRustConsole('canvas', `invoke missing for spotifyId=${cacheKey}`);
      canvasConsoleError('fetch_spotify_canvas invoke unavailable', { spotifyId: cacheKey });
      showCanvasToast('Canvas Spotify', 'Tauri invoke недоступен', false, 3000);
      clearPlayerCanvasLayer();
      return;
    }
    try {
      showCanvasToast('Canvas Spotify', `Ищу канвас для ${track.title || cacheKey}`, true);
      logRustConsole('canvas', `invoke fetch_spotify_canvas spotifyId=${cacheKey}`);
      canvasConsole('fetch_spotify_canvas start', { spotifyId: cacheKey, title: track.title || null });
      info = await inv('fetch_spotify_canvas', { track: track.spotifyId });
      logRustConsole('canvas', `invoke result spotifyId=${cacheKey} hasInfo=${!!info} type=${info?.canvasType || 'none'} proxied=${info?.proxiedCanvasUrl ? 'yes' : 'no'}`);
      canvasConsole('fetch_spotify_canvas result', {
        spotifyId: cacheKey,
        hasInfo: !!info,
        canvasType: info?.canvasType || null,
        canvasUrl: (info?.canvasUrl || '').slice(0, 200),
        proxiedCanvasUrl: (info?.proxiedCanvasUrl || '').slice(0, 200)
      });
      if (info) {
        showCanvasToast('Canvas Spotify', 'Ссылка на канвас получена', true);
        spotifyCanvasCache.set(cacheKey, info);
        track.spotifyCanvasUrl = info.canvasUrl || null;
        track.spotifyCanvasProxyUrl = info.proxiedCanvasUrl || null;
        track.spotifyCanvasType = info.canvasType || null;
      }
    } catch (e) {
      const errText = describeInvokeError(e);
      canvasConsoleError('fetch_spotify_canvas failed', { spotifyId: cacheKey, error: errText });
      logRustConsole('canvas', `invoke failed spotifyId=${cacheKey} err=${errText}`);
      showCanvasToast('Canvas Spotify', `Ошибка запроса: ${errText}`, false, 4200);
      info = null;
    }
  }

  if (reqId !== playerCanvasRequestId) {
    logRustConsole('canvas', `stale request req=${reqId} current=${playerCanvasRequestId}`);
    canvasConsoleWarn('stale canvas request', { reqId, currentRequestId: playerCanvasRequestId, spotifyId: cacheKey });
    return;
  }
  if (cur < 0 || T[cur] !== track) {
    logRustConsole('canvas', `track changed before apply req=${reqId} cur=${cur}`);
    canvasConsoleWarn('track changed before canvas apply', {
      reqId,
      cur,
      expectedSpotifyId: cacheKey,
      currentSpotifyId: T[cur]?.spotifyId || null
    });
    return;
  }
  if (!info || info.canvasType !== 'video') {
    logRustConsole('canvas', `no video canvas for spotifyId=${cacheKey}`);
    canvasConsoleWarn('no usable video canvas', {
      spotifyId: cacheKey,
      hasInfo: !!info,
      canvasType: info?.canvasType || null,
      canvasUrl: (info?.canvasUrl || '').slice(0, 200),
      proxiedCanvasUrl: (info?.proxiedCanvasUrl || '').slice(0, 200)
    });
    showCanvasToast('Canvas Spotify', 'Для трека нет video canvas', false, 2500);
    clearPlayerCanvasLayer();
    return;
  }
  canvasConsole('canvas ready for cached apply', {
    spotifyId: cacheKey,
    canvasType: info.canvasType || null,
    canvasUrl: (info.canvasUrl || '').slice(0, 200)
  });
  applyPlayerCanvasLayerCached(info);
}

async function scFetch(url) {
  const inv = getTauriInvoke();
  if (!inv) {
    console.error('[SC] Tauri invoke missing for:', url);
    return null;
  }
  try {
    return await inv('http_get', { url });
  } catch (e) {
    console.warn('[SC] http_get failed:', e);
    throw e;
  }
}

async function scFetchArt(url) {
  const inv = getTauriInvoke();
  if (typeof inv === 'function') {
    // Try Rust-side cover cache first (instant, no network)
    const cacheKey = 'art_' + url.replace(/[^a-zA-Z0-9]/g, '_').slice(0, 80);
    try {
      const cached = await inv('load_cached_cover_art', { trackId: cacheKey });
      if (cached) return cached;
    } catch (_) { }
    // Fetch from network
    try {
      const dataUrl = await inv('http_get_binary', { url });
      if (dataUrl) {
        // Cache in Rust filesystem for instant access next time
        try { inv('cache_cover_art', { trackId: cacheKey, dataUrl }); } catch (_) { }
        return dataUrl;
      }
    } catch (e) { }
  }
  return null;
}

async function scGetPlaylistJson(pl, clientId) {
  const playlistUrl = `https://api-v2.soundcloud.com/playlists/${pl.id}?client_id=${clientId}`;
  const text = await scFetch(playlistUrl);
  let json = text ? JSON.parse(text) : null;
  const directTracks = Array.isArray(json?.tracks) ? json.tracks : [];
  const expectedCount = Number(json?.track_count || pl?.track_count || 0) || 0;

  if (pl?.permalink_url && (!json || (expectedCount > directTracks.length && directTracks.length <= 5))) {
    try {
      const resolved = await scResolveEntity(pl.permalink_url);
      const resolvedJson = resolved?.json || null;
      const resolvedTracks = Array.isArray(resolvedJson?.tracks) ? resolvedJson.tracks : [];
      if (resolvedJson && resolvedTracks.length > directTracks.length) {
        json = resolvedJson;
      }
    } catch (e) { }
  }

  return json || { tracks: [] };
}

async function scFetchTracksByIds(trackIds, clientId) {
  const ids = Array.from(new Set((trackIds || []).map(id => String(id || '').trim()).filter(Boolean)));
  if (!ids.length) return [];
  const out = [];
  const chunkSize = 50;
  for (let i = 0; i < ids.length; i += chunkSize) {
    const chunk = ids.slice(i, i + chunkSize);
    const url = `https://api-v2.soundcloud.com/tracks?ids=${encodeURIComponent(chunk.join(','))}&client_id=${clientId}`;
    try {
      const text = await scFetch(url);
      if (!text) continue;
      const json = JSON.parse(text);
      if (Array.isArray(json)) out.push(...json);
      else if (Array.isArray(json.collection)) out.push(...json.collection);
    } catch (e) { }
  }
  return out;
}

async function scExpandPlaylistTracks(items, clientId) {
  const tracks = Array.isArray(items) ? items.slice() : [];
  if (!tracks.length) return tracks;

  const missingIds = tracks
    .filter(item => {
      if (!item) return false;
      const id = scTrackIdFromAny(item);
      if (!id) return false;
      const hasTitle = !!item.title;
      const hasUser = !!(item.user && item.user.username);
      const hasPermalink = !!item.permalink_url;
      return !(hasTitle && hasUser && hasPermalink);
    })
    .map(scTrackIdFromAny);

  if (!missingIds.length) return tracks;

  const fetched = await scFetchTracksByIds(missingIds, clientId);
  if (!fetched.length) return tracks;

  const fetchedById = new Map();
  fetched.forEach(item => {
    const id = scTrackIdFromAny(item);
    if (id) fetchedById.set(id, item);
  });

  return tracks.map(item => {
    const id = scTrackIdFromAny(item);
    if (!id) return item;
    const full = fetchedById.get(id);
    return full ? { ...item, ...full } : item;
  });
}

function scGetArtworkUrl(item) {
  if (!item) return '';
  if (item.artwork_url) return String(item.artwork_url).replace('-large', '-t500x500');
  if (item.user && item.user.avatar_url) return String(item.user.avatar_url).replace('-large', '-t500x500');
  return '';
}

function scBuildLightTrackFromJson(item, sourceUrl) {
  if (!item) return null;
  const scTrackId = scTrackIdFromAny(item);
  if (!scTrackId) return null;
  const permalinkUrl = (item.permalink_url && String(item.permalink_url)) || sourceUrl || '';
  const title = item.title || 'Unknown Title';
  const artist = item.user ? item.user.username : 'Unknown Artist';
  const duration = (Number(item.full_duration || item.duration || 0) || 0) / 1000;
  const artworkUrl = scGetArtworkUrl(item);

  let track = T.find(x => x.scTrackId === scTrackId) || null;
  if (!track) {
    track = {
      id: `sc_${scTrackId}`,
      scTrackId,
      file: null,
      path: sourceUrl || permalinkUrl || `soundcloud:${title || 'track'}`,
      sourceUrl: sourceUrl || permalinkUrl || '',
      streamUrl: '',
      url: '',
      title,
      artist,
      album: 'SoundCloud',
      duration,
      art: null,
      liked: false,
      isSoundCloud: true,
      _previewTransient: true
    };
    ensureTrackId(track);
    T.push(track);
  } else {
    if (!track.title || track.title === 'Unknown Title') track.title = title;
    if (!track.artist || track.artist === 'Unknown Artist') track.artist = artist;
    if (!track.duration) track.duration = duration;
    if (!track.sourceUrl) track.sourceUrl = sourceUrl || permalinkUrl || '';
    if (!track.path) track.path = sourceUrl || permalinkUrl || `soundcloud:${title || 'track'}`;
    track.isSoundCloud = true;
    track._previewTransient = true;
  }

  if (artworkUrl && !track._pendingArtworkUrl) track._pendingArtworkUrl = artworkUrl;
  const cachedArt = scReadCachedArt(scTrackId);
  if (cachedArt && !track.art) track.art = cachedArt;
  return track;
}

function scPersistArtCache(scTrackId, artDataUrl) {
  if (!scTrackId || !artDataUrl) return;
  try {
    const rawArt = localStorage.getItem(SC_ART_CACHE_KEY);
    const map = rawArt ? JSON.parse(rawArt) || {} : {};
    map[scTrackId] = artDataUrl;
    localStorage.setItem(SC_ART_CACHE_KEY, JSON.stringify(map));
  } catch (e) { }
}

function scReadCachedArt(scTrackId) {
  if (!scTrackId) return null;
  try {
    const rawArt = localStorage.getItem(SC_ART_CACHE_KEY);
    const map = rawArt ? JSON.parse(rawArt) || {} : {};
    return map[scTrackId] || null;
  } catch (e) {
    return null;
  }
}

function syncTrackArtworkUI(track) {
  if (!track || !track.id || !track.art) return;
  const idx = T.findIndex(x => x.id === track.id);
  if (idx < 0) return;

  document.querySelectorAll(`.ti[data-idx="${idx}"] .ta`).forEach(ta => {
    ta.innerHTML = `<img src="${track.art}" alt="">`;
  });

  if (cur === idx) upUI(track);
}

function scQueueTrackArtHydration(track, info) {
  if (!track || track.art || track._artLoading || !info || !info.artworkUrl) return;
  track._artLoading = true;
  setTimeout(async () => {
    try {
      const artDataUrl = await scFetchArt(info.artworkUrl);
      if (!artDataUrl || track.art) return;
      track.art = artDataUrl;
      if (track.scTrackId) scPersistArtCache(track.scTrackId, artDataUrl);
      syncTrackArtworkUI(track);
      saveT();
    } catch (e) {
    } finally {
      track._artLoading = false;
    }
  }, 0);
}

async function scHydratePlaylistArtworkInBackground(entries) {
  if (!Array.isArray(entries) || !entries.length) return;
  for (const entry of entries) {
    const track = entry?.track || null;
    const artworkUrl = entry?.artworkUrl || track?._pendingArtworkUrl || '';
    if (track && !track.art && artworkUrl) {
      scQueueTrackArtHydration(track, { artworkUrl });
    }
    await new Promise(resolve => setTimeout(resolve, 20));
  }
}

async function scGetClientId() {
  if (scClientId) return scClientId;
  // Java app fallback
  scClientId = 'DAXAfBNYHaWC72FM2w6Jvh84R96RfMYP';
  console.log('[SC] Using Java-aligned fallback client_id');

  try {
    const html = await scFetch('https://soundcloud.com');
    if (html) {
      // Find all JS bundles
      const scriptMatches = [...html.matchAll(/src="(https:\/\/a-v2\.sndcdn\.com\/assets\/[^"]+\.js)"/g)];
      // Check last few bundles first (usually where the ID sits)
      for (const m of scriptMatches.reverse().slice(0, 10)) {
        const js = await scFetch(m[1]);
        if (!js) continue;
        const idMatch = js.match(/client_id[:=]"([a-zA-Z0-9]{32})"/);
        if (idMatch) {
          scClientId = idMatch[1];
          console.log('[SC] client_id extracted:', scClientId);
          return scClientId;
        }
      }
    }
  } catch (e) { console.warn('[SC] client_id extraction failed', e); }
  return scClientId;
}

async function scResolveEntity(soundCloudUrl) {
  const clientId = await scGetClientId();
  const resolveUrl = `https://api-v2.soundcloud.com/resolve?url=${encodeURIComponent(soundCloudUrl)}&client_id=${clientId}`;
  console.log('[SC] Resolving:', resolveUrl);
  const text = await scFetch(resolveUrl);
  if (!text) {
    const inv = getTauriInvoke();
    if (!inv) return 'INVOKE_MISSING';
    return 'EMPTY_RESPONSE';
  }
  try {
    const json = JSON.parse(text);
    return { json, clientId };
  } catch (e) { console.error('[SC] resolve parse error', e); return null; }
}

async function scResolveTrack(soundCloudUrl) {
  const entity = await scResolveEntity(soundCloudUrl);
  if (!entity || typeof entity === 'string') return entity;
  const kind = entity.json.kind || '';
  if (kind === 'playlist' || kind === 'system-playlist') {
    if (entity.json.tracks && entity.json.tracks.length) {
      for (const t of entity.json.tracks) {
        if (t.id) {
          const byId = await scResolveTrackById(String(t.id), entity.clientId);
          if (byId && byId.streamUrl) return byId;
        }
      }
    }
    return null;
  }
  return await scExtractTrackInfo(entity.json, entity.clientId);
}

async function scResolveTrackById(trackId, clientId) {
  const url = `https://api-v2.soundcloud.com/tracks/${trackId}?client_id=${clientId}`;
  const text = await scFetch(url);
  if (!text) return null;
  try {
    const json = JSON.parse(text);
    return await scExtractTrackInfo(json, clientId);
  } catch (e) { console.error('[SC] trackById parse error', e); return null; }
}

async function scResolvePlaylistTrackInfo(item, clientId) {
  const trackId = scTrackIdFromAny(item);
  if (!trackId) return null;
  const hasTranscodings = !!(item.media && Array.isArray(item.media.transcodings) && item.media.transcodings.length);
  if (hasTranscodings) {
    const info = await scExtractTrackInfo(item, clientId);
    if (info && info.streamUrl) return info;
  }
  return await scResolveTrackById(trackId, clientId);
}

async function scExtractTrackInfo(json, clientId) {
  const scTrackId = scTrackIdFromAny(json);
  const permalinkUrl = (json && json.permalink_url) ? String(json.permalink_url) : '';
  const title = json.title || 'Unknown Title';
  const artist = json.user ? json.user.username : 'Unknown Artist';
  let artworkUrl = '';
  if (json.artwork_url) {
    artworkUrl = json.artwork_url.replace('-large', '-t500x500');
  } else if (json.user && json.user.avatar_url) {
    artworkUrl = json.user.avatar_url.replace('-large', '-t500x500');
  }
  const durationMs = json.full_duration || json.duration || 0;

  let streamUrl = null;
  if (json.media && json.media.transcodings) {
    const transcodings = json.media.transcodings;
    // Pass 1: progressive only
    for (const trans of transcodings) {
      if (trans.format && trans.format.protocol === 'progressive') {
        streamUrl = await scFetchStreamUrl(trans.url + '?client_id=' + clientId);
        if (streamUrl) break;
      }
    }
    // Pass 2: fallback to any
    if (!streamUrl) {
      for (const trans of transcodings) {
        streamUrl = await scFetchStreamUrl(trans.url + '?client_id=' + clientId);
        if (streamUrl) break;
      }
    }
  }

  return { scTrackId, permalinkUrl, title, artist, artworkUrl, streamUrl, durationMs };
}

async function scFetchStreamUrl(urlWithClientId) {
  const text = await scFetch(urlWithClientId);
  if (!text) return null;
  try {
    const json = JSON.parse(text);
    return json.url || null;
  } catch (e) { return null; }
}

// Toast helpers
function scToast(title, sub, loading) {
  const el = document.getElementById('sc-toast');
  document.getElementById('sc-toast-title').textContent = title;
  document.getElementById('sc-toast-sub').textContent = sub;
  document.getElementById('sc-toast-bar').style.display = loading ? '' : 'none';
  el.classList.add('show');
}
function scToastHide() {
  document.getElementById('sc-toast').classList.remove('show');
}

let canvasToastHideTimer = null;
function showCanvasToast(title, sub, loading, autoHideMs) {
  if (canvasToastHideTimer) {
    clearTimeout(canvasToastHideTimer);
    canvasToastHideTimer = null;
  }
  scToast(title, sub, loading);
  if (!loading && autoHideMs) {
    canvasToastHideTimer = setTimeout(() => {
      scToastHide();
      canvasToastHideTimer = null;
    }, autoHideMs);
  }
}

async function scReadClipboardText() {
  try {
    const text = await navigator.clipboard.readText();
    if (text && text.trim()) return text.trim();
  } catch (e) { }
  const manual = prompt('Вставьте ссылку на SoundCloud', '');
  return (manual || '').trim();
}

function addTrackToPlaylistById(trackId, playlistId) {
  const pl = P.find(x => x.id === playlistId);
  if (!pl || pl.locked) return;
  pl.trackIds = Array.from(new Set(pl.trackIds || []));
  if (!pl.trackIds.includes(trackId)) pl.trackIds.push(trackId);
}

async function scAddTrackFromInfo(info, sourceUrl, spotifyId, options) {
  if (!info || !info.streamUrl) return null;
  const opts = options || {};
  const scTrackId = String(info.scTrackId || '');
  let artDataUrl = scReadCachedArt(scTrackId);
  if (!artDataUrl && info.artworkUrl && !opts.deferArt) artDataUrl = await scFetchArt(info.artworkUrl);
  const port = await scGetProxyPort();
  const proxiedUrl = `http://127.0.0.1:${port}/stream?url=${encodeURIComponent(info.streamUrl)}`;
  let track = null;
  if (scTrackId) track = T.find(x => x.scTrackId === scTrackId) || null;
  if (!track) track = T.find(x => x.streamUrl === info.streamUrl || x.url === proxiedUrl) || null;
  if (track) {
    if (!track.art && artDataUrl) track.art = artDataUrl;
    if (!track.art && opts.deferArt) scQueueTrackArtHydration(track, info);
    if (!opts.transientPreview) delete track._previewTransient;
    return track;
  }
  track = {
    id: scTrackId ? `sc_${scTrackId}` : undefined,
    scTrackId,
    file: null,
    path: sourceUrl || info.permalinkUrl || `soundcloud:${info.title || 'track'}`,
    sourceUrl: sourceUrl || info.permalinkUrl || '',
    streamUrl: info.streamUrl,
    url: proxiedUrl,
    title: info.title || 'Unknown Title',
    artist: info.artist || 'Unknown Artist',
    album: 'SoundCloud',
    duration: (Number(info.durationMs) || 0) / 1000,
    art: artDataUrl,
    liked: false,
    isSoundCloud: true,
    spotifyId: spotifyId || null,
    _previewTransient: !!opts.transientPreview
  };
  ensureTrackId(track);
  if (scTrackId && artDataUrl) scPersistArtCache(scTrackId, artDataUrl);
  T.push(track);
  if (!track.art && opts.deferArt) scQueueTrackArtHydration(track, info);
  if (!opts.transientPreview) delete track._previewTransient;
  return track;
}

async function scRefreshTrackForPlayback(track, forceArt) {
  if (!track || !track.isSoundCloud) return false;
  try {
    let info = null;
    if (track.scTrackId) {
      const clientId = await scGetClientId();
      info = await scResolveTrackById(String(track.scTrackId), clientId);
    } else {
      const source = track.sourceUrl || track.path || '';
      if (!source.includes('soundcloud.com')) return false;
      info = await scResolveTrack(source);
    }
    if (!info || typeof info === 'string' || !info.streamUrl) return false;
    const port = await scGetProxyPort();
    if (info.scTrackId && !track.scTrackId) {
      track.scTrackId = String(info.scTrackId);
      track.id = `sc_${track.scTrackId}`;
    }
    track.streamUrl = info.streamUrl;
    track.url = `http://127.0.0.1:${port}/stream?url=${encodeURIComponent(info.streamUrl)}`;
    if (info.title) track.title = info.title;
    if (info.artist) track.artist = info.artist;
    if (info.durationMs) track.duration = (Number(info.durationMs) || 0) / 1000;
    if (forceArt && info.artworkUrl && !track.art) {
      const art = await scFetchArt(info.artworkUrl);
      if (art) track.art = art;
    }
    if (track.scTrackId && track.art) {
      try {
        const rawArt = localStorage.getItem(SC_ART_CACHE_KEY);
        const map = rawArt ? JSON.parse(rawArt) || {} : {};
        map[track.scTrackId] = track.art;
        localStorage.setItem(SC_ART_CACHE_KEY, JSON.stringify(map));
      } catch (e) { }
    }
    ensureTrackId(track);
    saveSoundCloudCache();
    rl();
    return true;
  } catch (e) {
    console.warn('[SC] refresh failed', e);
    return false;
  }
}

async function scLoadFromClipboard() {
  const clipText = await scReadClipboardText();
  if (!clipText) {
    scToast('Пустая ссылка', 'Буфер пуст или ввод отменен', false);
    setTimeout(scToastHide, 2500);
    return;
  }
  if (!clipText.includes('soundcloud.com')) {
    scToast('Не SoundCloud', 'В буфере нет ссылки на SoundCloud', false);
    setTimeout(scToastHide, 2500);
    return;
  }

  scToast('Загрузка трека...', clipText.substring(0, 50), true);

  try {
    const info = await scResolveTrack(clipText);
    if (!info || typeof info === 'string') {
      const detail = typeof info === 'string' ? ` (${info})` : ' (null/unexpected)';
      scToast('Ошибка', 'Не удалось распознать ссылку' + detail, false);
      setTimeout(scToastHide, 3000);
      return;
    }
    if (!info.streamUrl) {
      scToast('Ошибка', 'Для этого трека нет доступного стрима', false);
      setTimeout(scToastHide, 3000);
      return;
    }

    const track = await scAddTrackFromInfo(info, info.permalinkUrl || clipText);
    if (!track) {
      scToast('Ошибка', 'Не удалось добавить трек', false);
      setTimeout(scToastHide, 3000);
      return;
    }
    rl();
    renderPlaylists();
    saveSoundCloudCache();
    play(T.indexOf(track));

    scToast(info.title, info.artist + ' • SoundCloud', false);
    setTimeout(scToastHide, 3000);
  } catch (e) {
    console.error('[SC] load error', e);
    scToast('Ошибка загрузки', String(e).substring(0, 100), false);
    setTimeout(scToastHide, 5000);
  }
}

async function importSoundCloudPlaylistByPrompt() {
  const clipText = await scReadClipboardText();
  if (!clipText || !clipText.includes('soundcloud.com')) {
    scToast('Не SoundCloud', 'Вставь ссылку на плейлист', false);
    setTimeout(scToastHide, 2500);
    return;
  }
  scToast('Импорт плейлиста...', clipText.substring(0, 50), true);
  try {
    const entity = await scResolveEntity(clipText);
    if (!entity || typeof entity === 'string') {
      scToast('Ошибка', 'Плейлист не распознан', false);
      setTimeout(scToastHide, 3000);
      return;
    }
    const json = entity.json || {};
    const kind = json.kind || '';
    if (!(kind === 'playlist' || kind === 'system-playlist')) {
      scToast('Это не плейлист', 'Ссылка ведёт на трек', false);
      setTimeout(scToastHide, 3000);
      return;
    }
    const name = (json.title || 'SoundCloud Playlist').trim();
    let plItem = P.find(x => x.source === 'soundcloud' && x.sourceUrl === clipText);
    if (!plItem) {
      plItem = createPlaylist(name, json.description || '', { source: 'soundcloud', sourceUrl: clipText, color: '255,119,0' });
    }
    const tracks = Array.isArray(json.tracks) ? json.tracks : [];
    let added = 0;
    for (const tr of tracks) {
      if (!tr || !tr.id) continue;
      const info = await scResolveTrackById(String(tr.id), entity.clientId);
      if (!info || !info.streamUrl) continue;
      const trUrl = (tr.permalink_url && String(tr.permalink_url)) || info.permalinkUrl || clipText;
      const addedTrack = await scAddTrackFromInfo(info, trUrl);
      if (!addedTrack) continue;
      addTrackToPlaylistById(addedTrack.id, plItem.id);
      added++;
    }
    savePlaylistsCache();
    saveSoundCloudCache();
    activePlaylistId = plItem.id;
    closePlaylistCreate();
    rl();
    scToast(plItem.name, `Импортировано: ${added} треков`, false);
    setTimeout(scToastHide, 3500);
  } catch (e) {
    console.error('[SC playlist import]', e);
    scToast('Ошибка импорта', String(e).substring(0, 100), false);
    setTimeout(scToastHide, 4000);
  }
}

//  ADD BUTTON: click = file, long-press = SoundCloud 
(function () {
  const btn = document.getElementById('addbtn');
  const fi = document.getElementById('fi');
  let lpTimer = null;
  let wasLongPress = false;

  function onDown() {
    wasLongPress = false;
    btn.classList.remove('lp');
    lpTimer = setTimeout(() => {
      wasLongPress = true;
      btn.classList.add('lp');
      btn.style.transform = 'scale(0.85)';
      setTimeout(() => btn.style.transform = '', 150);
    }, 500);
  }

  async function onUp() {
    clearTimeout(lpTimer);
    setTimeout(() => btn.classList.remove('lp'), 300);
    if (!wasLongPress) {
      fi.click();
    } else {
      const mode = prompt('SoundCloud: 1 - трек, 2 - плейлист', '1');
      if ((mode || '').trim() === '2') await importSoundCloudPlaylistByPrompt();
      else await scLoadFromClipboard();
    }
    wasLongPress = false;
  }

  function onCancel() {
    clearTimeout(lpTimer);
    btn.classList.remove('lp');
    wasLongPress = false;
  }

  btn.addEventListener('mousedown', onDown);
  btn.addEventListener('mouseup', () => { void onUp(); });
  btn.addEventListener('mouseleave', onCancel);
  btn.addEventListener('touchstart', e => { e.preventDefault(); onDown(); }, { passive: false });
  btn.addEventListener('touchend', e => { e.preventDefault(); void onUp(); });
  btn.addEventListener('touchcancel', onCancel);
})();

function parseCsvLine(line) {
  const out = [];
  let cur = '';
  let inQuotes = false;
  for (let i = 0; i < line.length; i++) {
    const ch = line[i];
    if (ch === '"') {
      if (inQuotes && line[i + 1] === '"') {
        cur += '"';
        i++;
      } else {
        inQuotes = !inQuotes;
      }
      continue;
    }
    if (ch === ',' && !inQuotes) {
      out.push(cur);
      cur = '';
      continue;
    }
    cur += ch;
  }
  out.push(cur);
  return out.map(v => String(v || '').trim());
}

function parseSpotifyCsvRows(csv) {
  const lines = String(csv || '')
    .split(/\r?\n/)
    .map(l => l.trim())
    .filter(Boolean);
  if (lines.length < 2) return [];

  const header = parseCsvLine(lines[0]);
  const headerMap = new Map(header.map((name, idx) => [String(name || '').toLowerCase(), idx]));
  const getValue = (cols, key) => {
    const idx = headerMap.get(key);
    return idx == null ? '' : String(cols[idx] || '').trim();
  };

  const isSpotifyCsv = headerMap.has('track uri') || headerMap.has('track name');
  if (!isSpotifyCsv) return [];

  return lines.slice(1).map(line => {
    const cols = parseCsvLine(line);
    const trackUri = getValue(cols, 'track uri');
    const spotifyId = trackUri.startsWith('spotify:track:') ? trackUri.split(':').pop() : '';
    return {
      spotifyId,
      spotifyUri: trackUri || (spotifyId ? `spotify:track:${spotifyId}` : ''),
      title: getValue(cols, 'track name'),
      artist: getValue(cols, 'artist name(s)') || getValue(cols, 'artist name'),
      album: getValue(cols, 'album name'),
      art: getValue(cols, 'album image url'),
      durationMs: Number(getValue(cols, 'track duration (ms)')) || 0,
      previewUrl: getValue(cols, 'track preview url')
    };
  }).filter(row => row.spotifyId && row.title);
}

function upsertSpotifyTrackFromCsvRow(row, insertIdx) {
  if (!row || !row.spotifyId || !row.title) return null;
  const spotifyId = String(row.spotifyId);
  let track = T.find(t => t.spotifyId === spotifyId) || null;

  if (!track) {
    track = {
      id: `sp_${spotifyId}`,
      spotifyId,
      spotifyUri: row.spotifyUri || `spotify:track:${spotifyId}`,
      spotifyWebUrl: `https://open.spotify.com/track/${spotifyId}`,
      file: null,
      path: row.spotifyUri || `spotify:track:${spotifyId}`,
      sourceUrl: `https://open.spotify.com/track/${spotifyId}`,
      url: '',
      streamUrl: '',
      spotifyPreviewUrl: row.previewUrl || '',
      title: row.title || 'Unknown Title',
      artist: row.artist || 'Unknown Artist',
      album: row.album || '',
      duration: (Number(row.durationMs) || 0) / 1000,
      art: row.art || null,
      liked: false,
      isSpotify: true,
      _insertIdx: insertIdx
    };
    ensureTrackId(track);
    T.push(track);
    return track;
  }

  track.spotifyId = spotifyId;
  track.spotifyUri = track.spotifyUri || row.spotifyUri || `spotify:track:${spotifyId}`;
  track.spotifyWebUrl = track.spotifyWebUrl || `https://open.spotify.com/track/${spotifyId}`;
  track.path = track.path || track.spotifyUri || `spotify:track:${spotifyId}`;
  track.sourceUrl = track.sourceUrl || `https://open.spotify.com/track/${spotifyId}`;
  if (!track.title || track.title === 'Unknown Title') track.title = row.title;
  if (!track.artist || track.artist === 'Unknown Artist') track.artist = row.artist || 'Unknown Artist';
  if (!track.album) track.album = row.album || '';
  if (!track.duration) track.duration = (Number(row.durationMs) || 0) / 1000;
  if (!track.art && row.art) track.art = row.art;
  if (!track.spotifyPreviewUrl && row.previewUrl) track.spotifyPreviewUrl = row.previewUrl;
  if (!track._insertIdx && track._insertIdx !== 0) track._insertIdx = insertIdx;
  if (track.isSoundCloud && !track.scTrackId) {
    track.isSoundCloud = false;
  }
  track.isSpotify = true;
  return track;
}

function getTauriConvertFileSrc() {
  if (window.__TAURI__) {
    if (window.__TAURI__.core && window.__TAURI__.core.convertFileSrc) return window.__TAURI__.core.convertFileSrc;
    if (window.__TAURI__.convertFileSrc) return window.__TAURI__.convertFileSrc;
  }
  return null;
}

function toPlayableUrl(path) {
  const convert = getTauriConvertFileSrc();
  if (typeof convert === 'function') return convert(path);
  return `file://${path}`;
}

function parseTagsForTrack(track) {
  fetch(track.url)
    .then(r => r.blob())
    .then(blob => readTagsFromBlob(blob))
    .then(tags => {
      applyParsedTags(track, tags || {});
      rl();
      if (cur >= 0 && T[cur] === track) {
        upUI(track);
        upMini(track);
        extractColor(track, playEpoch);
      }
    })
    .catch(() => { });
}

function loadDurationForTrack(track) {
  const a = new Audio(track.url);
  a.addEventListener('loadedmetadata', () => {
    if (!isNaN(a.duration) && isFinite(a.duration)) {
      track.duration = a.duration;
      rl();
    }
  }, { once: true });
}

function hydrateTracksFromPaths(paths) {
  if (!Array.isArray(paths) || !paths.length) return;
  paths.forEach(path => {
    const url = toPlayableUrl(path);
    if (T.some(x => x.url === url || x.path === path)) return;
    const fileName = (path.split(/[\\/]/).pop() || 'Unknown Track').replace(/\.[^/.]+$/, '');
    const track = {
      file: null,
      path,
      url,
      title: fileName,
      artist: 'Unknown Artist',
      album: '',
      duration: 0,
      art: null,
      liked: false
    };
    ensureTrackId(track);
    T.push(track);
    parseTagsForTrack(track);
    loadDurationForTrack(track);
  });
  rl();
}

function hydrateTracksFromScan(tracks) {
  if (!Array.isArray(tracks) || !tracks.length) return;
  tracks.forEach(t => {
    const path = t.path;
    const url = toPlayableUrl(path);
    if (T.some(x => x.url === url || x.path === path)) return;
    const fileName = (path.split(/[\\/]/).pop() || 'Unknown Track').replace(/\.[^/.]+$/, '');
    const track = {
      file: null,
      path,
      url,
      title: t.title || fileName,
      artist: t.artist || 'Unknown Artist',
      album: t.album || '',
      duration: Number(t.duration) || 0,
      art: t.coverDataUrl || null,
      liked: false
    };
    ensureTrackId(track);
    T.push(track);
    if (!track.art || track.artist === 'Unknown Artist') parseTagsForTrack(track);
    if (!track.duration) loadDurationForTrack(track);
  });
  rl();
}

async function syncMusicPathCache() {
  const inv = getTauriInvoke();
  if (typeof inv !== 'function') {
    console.error('[music-cache] sync failed: invoke missing');
    return;
  }
  try {
    const paths = await inv('rescan_music_paths');
    localStorage.setItem('music_paths_cache', JSON.stringify(paths || []));
    localStorage.setItem('music_paths_cache_updated_at', String(Date.now()));
    const tracks = await inv('get_scanned_tracks');
    hydrateTracksFromScan(tracks || []);
    if (!tracks || !tracks.length) hydrateTracksFromPaths(paths || []);
    console.log('[music-cache] scanned:', (paths || []).length);
  } catch (err) {
    console.warn('[music-cache] scan failed', err);
    try {
      const cached = await inv('get_cached_music_paths');
      localStorage.setItem('music_paths_cache', JSON.stringify(cached || []));
      localStorage.setItem('music_paths_cache_updated_at', String(Date.now()));
      const tracks = await inv('get_scanned_tracks');
      hydrateTracksFromScan(tracks || []);
      if (!tracks || !tracks.length) hydrateTracksFromPaths(cached || []);
    } catch (e) { }
  }
}
// Автоскан локальной музыки отключен по запросу.
// --- SPOTIFY INTEGRATION (PKCE) ---
// Load Spotify token with proper error handling
let spotifyToken = null;

async function loadSpotifyTokenFromStorage() {
  try {
    const inv = getTauriInvoke();
    if (!inv) throw new Error("Tauri invoke not available");

    spotifyToken = await inv('load_spotify_token') || null;

    if (spotifyToken) {
      console.log('[Spotify] Token loaded from Tauri storage, length:', spotifyToken.length);
      addDebugLog(`✓ Токен загружен (${spotifyToken.length} символов)`);
    } else {
      console.log('[Spotify] No token found, trying sp_dc auto-refresh...');
      addDebugLog('Токен не найден, пробуем sp_dc...');
      // Тихо пробуем получить токен через sp_dc (если кука сохранена)
      try {
        const newToken = await inv('fetch_spotify_token_from_sp_dc');
        if (newToken && newToken.length > 10) {
          spotifyToken = newToken;
          console.log('[Spotify] ✓ Token auto-refreshed via sp_dc, length:', newToken.length);
          addDebugLog(`✓ Токен получен через sp_dc (${newToken.length} символов)`);
        }
      } catch (_) {
        // sp_dc не сохранён или не работает — это нормально
        addDebugLog('✗ Токен не найден');
      }
    }
  } catch (e) {
    console.error('[Spotify] Failed to load token:', e);
    addDebugLog('✗ Ошибка загрузки: ' + e.message);
    spotifyToken = null;
  }
  return spotifyToken;
}

async function saveSpotifyTokenToStorage(token) {
  try {
    if (!token || token.length < 10) {
      console.error('[Spotify] Invalid token, not saving:', token?.length);
      return false;
    }

    const inv = getTauriInvoke();
    if (!inv) throw new Error("Tauri invoke not available");

    await inv('save_spotify_token', { token });
    spotifyToken = token;
    console.log('[Spotify] Token saved to Tauri storage, length:', token.length);
    addDebugLog(`✓ Токен сохранен (${token.length} символов)`);
    return true;
  } catch (e) {
    console.error('[Spotify] Failed to save token:', e);
    addDebugLog('✗ Ошибка сохранения: ' + e.message);
    return false;
  }
}

async function initializeSpotifyStatus() {
  // Reload token from storage on every call to ensure freshness
  await loadSpotifyTokenFromStorage();

  if (spotifyToken && spotifyToken.length > 10) {
    console.log('[Spotify] Token found, restoring status');
    document.getElementById('sp-status').textContent = 'Подключено';
    const btn = document.getElementById('sp-main-btn');
    if (btn) {
      btn.textContent = 'Управление';
      btn.onclick = openSpotifyManager;
    }
  } else {
    console.log('[Spotify] No valid token, showing login button');
    document.getElementById('sp-status').textContent = 'Не подключено';
    const btn = document.getElementById('sp-main-btn');
    if (btn) {
      btn.textContent = 'Подключить';
      btn.onclick = loginSpotify;
    }
  }
}

async function loginSpotify() {
  // Всегда используем нативный Tauri WebView — единственный способ,
  // при котором fetch('/get_access_token') выполняется в контексте
  // open.spotify.com и имеет доступ к sp_dc cookie.
  // SpotifyLoginOverlay (iframe-путь) принципиально не работает:
  // tauriFetch — это отдельный Rust HTTP клиент без кук WebView.
  const inv = getTauriInvoke();
  if (!inv) {
    console.error('[Spotify] Tauri invoke not available');
    return;
  }
  try {
    scToast('Spotify Login', 'Войдите в свой аккаунт Spotify в открывшемся окне', true);
    await inv('start_spotify_login');
  } catch (e) {
    console.error('[Spotify] Auth start failed', e);
    scToast('Ошибка', 'Не удалось открыть окно авторизации: ' + e, false);
  }
}

function initSpotify() {
  if (!window.__TAURI__) return;
  const { listen } = window.__TAURI__.event;

  // Listen for the access token emitted by Rust after successful login
  listen('spotify-auth-token', async (event) => {
    const newToken = event.payload;

    if (!newToken || newToken.length < 10) {
      console.error('[Spotify] Invalid token received:', newToken);
      scToast('Ошибка', 'Не удалось получить токен. Попробуйте заново.', false);
      return;
    }

    console.log('[Spotify] ✓ Получен токен, обновляем статус...');

    await saveSpotifyTokenToStorage(newToken);
    await initializeSpotifyStatus();

    scToast('Spotify Connected ✓', 'Авторизация успешна!', false);
    setTimeout(scToastHide, 3000);

    document.getElementById('sp-status').textContent = 'Подключено';
    const btn = document.getElementById('sp-main-btn');
    if (btn) { btn.textContent = 'Управление'; btn.onclick = openSpotifyManager; }

    // На десктопе окно уже закрыто Rust-ом через destroy_login_window.
    // На Android явно вызываем close_spotify_login чтобы сбросить состояние.
    const inv = getTauriInvoke();
    if (inv) {
      inv('close_spotify_login').catch(e => console.warn('[Spotify] Close window error:', e));
    }

    openSpotifyManager();
  });

  // Android: Rust эмитит spotify-login-close когда WebView очищен
  // (на Android нельзя закрыть WebviewWindow программно — destroy_login_window
  // очищает контент через eval и посылает этот ивент).
  listen('spotify-login-close', () => {
    console.log('[Spotify] Login window closed (Android)');
    initializeSpotifyStatus();
  });

  // Listen for CSV export from Exportify
  listen('spotify-import-csv', async (event) => {
    try {
      importCsvData(event.payload);
      const inv = getTauriInvoke();
      if (inv) inv('close_spotify_login').catch(e => console.warn(e));
    } catch (e) {
      console.error('[Spotify] CSV import error:', e);
      scToast('Ошибка', 'Не удалось импортировать CSV', false);
    }
  });

  // Android: Rust не может закрыть WebviewWindow напрямую,
  // поэтому эмитит это событие — мы пробуем закрыть через invoke
  listen('spotify-login-close', async () => {
    console.log('[Spotify] spotify-login-close received, closing login window...');
    const inv = getTauriInvoke();
    if (inv) {
      try {
        await inv('close_spotify_login');
      } catch (e) {
        console.warn('[Spotify] close_spotify_login failed:', e);
      }
    }
  });
}

// === РУЧНОЙ ВВОД SP_DC КУКИ ===
async function saveSpDcCookie() {
  const input = document.getElementById('sp-dc-input');
  if (!input) return;

  const spDcValue = input.value.trim();
  if (!spDcValue || spDcValue.length < 10) {
    scToast('Ошибка', 'Введите корректное значение sp_dc куки', false);
    return;
  }

  try {
    const inv = getTauriInvoke();
    if (!inv) throw new Error("Tauri invoke not available");

    // Сохраняем sp_dc (или Bearer токен напрямую)
    const saveResult = await inv('save_spotify_sp_dc', { spDc: spDcValue });
    console.log('[Spotify] save result:', saveResult);
    input.value = '';

    if (saveResult === 'bearer') {
      // Прямой Bearer токен — загружаем его из файла, fetch не нужен
      scToast('Spotify', 'Загружаем токен...', true);
      const savedToken = await inv('load_spotify_token');
      if (savedToken && savedToken.length > 10) {
        spotifyToken = savedToken;
        addDebugLog(`✓ Прямой Bearer токен сохранён (${savedToken.length} символов)`);
        await initializeSpotifyStatus();
        scToast('Spotify ✓', 'Токен сохранён', false);
        setTimeout(scToastHide, 3000);
      } else {
        scToast('Ошибка', 'Не удалось загрузить сохранённый токен', false);
      }
    } else {
      // sp_dc — получаем токен через Rust-запрос
      scToast('Spotify', 'Получаем токен...', true);
      await refreshSpotifyTokenFromSpDc();
    }
  } catch (e) {
    console.error('[Spotify] Failed to save sp_dc:', e);
    scToast('Ошибка', 'Не удалось сохранить sp_dc: ' + String(e.message ?? e), false);
  }
}

async function fetchSpotifyTokenFromSpDc() {
  return refreshSpotifyTokenFromSpDc();
}

// Получает свежий токен через sp_dc прямо из Rust (без Python скрипта).
// Вызывается: после сохранения sp_dc, при 401 ошибке, при старте приложения.
async function refreshSpotifyTokenFromSpDc() {
  const inv = getTauriInvoke();
  if (!inv) return null;

  try {
    console.log('[Spotify] Refreshing token via sp_dc...');
    const newToken = await inv('fetch_spotify_token_from_sp_dc');

    if (!newToken || newToken.length < 10) {
      throw new Error('Пустой токен в ответе');
    }

    spotifyToken = newToken;
    console.log('[Spotify] ✓ Token refreshed via sp_dc, length:', newToken.length);
    addDebugLog(`✓ Токен обновлён через sp_dc (${newToken.length} символов)`);

    await initializeSpotifyStatus();
    scToast('Spotify ✓', 'Токен получен и сохранён', false);
    setTimeout(scToastHide, 3000);
    return newToken;
  } catch (e) {
    const msg = String(e.message ?? e);
    console.error('[Spotify] Failed to refresh token via sp_dc:', msg);
    addDebugLog('✗ Ошибка обновления токена: ' + msg);
    scToast('Ошибка', 'Не удалось получить токен: ' + msg, false);
    return null;
  }
}

// Функция для диагностики (вызывается кнопкой)
async function diagnosticSpotify() {
  console.log('[Spotify] === НАЧАЛО ДИАГНОСТИКИ ===');
  console.log('[Spotify] Текущий токен:', spotifyToken ? spotifyToken.substring(0, 20) + '...' : 'ОТСУТСТВУЕТ');
  
  const inv = getTauriInvoke();
  if (inv) {
    try {
      const savedToken = await inv('load_spotify_token');
      console.log('[Spotify] Сохраненный токен:', savedToken ? savedToken.substring(0, 20) + '...' : 'НЕТ');
    } catch (e) {
      console.error('[Spotify] Ошибка загрузки токена:', e);
    }
  }
  
  scToast('Диагностика', 'Проверьте консоль (F12)', false);
}

function handleCsvManual() {
  document.getElementById('sp-csv-file').click();
}

async function handleCsvFile(input) {
  const file = input.files[0];
  if (!file) return;
  const reader = new FileReader();
  reader.onload = (e) => importCsvData(e.target.result, file.name.replace(/\.[^/.]+$/, ''));
  reader.readAsText(file);
  input.value = '';
}

async function diagnosticSpotify() {
  console.log('[Diagnostic] Starting Spotify diagnostic...');

  let result = '=== ДИАГНОСТИКА SPOTIFY ===\n\n';

  // Проверка 1: Токен
  const token = localStorage.getItem('spotify_token') || spotifyToken;
  if (token) {
    result += `✅ Токен найден\n`;
    result += `   Длина: ${token.length} символов\n`;
    result += `   Начало: ${token.substring(0, 20)}...\n\n`;
  } else {
    result += `❌ Токен НЕ найден\n`;
    result += `   Решение: Нажми "Подключить" и введи пароль Spotify\n\n`;
    alert(result);
    return;
  }

  // Проверка 2: Тестовый запрос к API
  result += 'Выполняю тестовый запрос к Spicy Lyrics API...\n\n';

  try {
    const inv = getTauriInvoke();
    if (!inv) throw new Error("No Tauri Invoke");

    // Используем популярный трек для теста
    const testSpotifyId = '3n3Ppam7vgaVa1iaRUc9Lp'; // "I Will Survive" - Gloria Gaynor (для теста)

    console.log('[Diagnostic] Sending test request to fetch_spicy_lyrics...');
    const res = await inv('fetch_spicy_lyrics', {
      spotifyId: testSpotifyId,
      token: token
    });

    const data = JSON.parse(res);
    result += `✅ Запрос успешен (HTTP 200)\n`;
    result += `   Ответ получен, размер: ${res.length} байт\n\n`;

    const queryItems = Array.isArray(data?.queries) ? data.queries : [];
    const lyricsJob =
      queryItems.find(q => q?.operation === 'lyrics' && q?.result?.data) ||
      queryItems.find(q => q?.result?.data) ||
      null;
    const lyrics = lyricsJob?.result?.data;
    if (lyrics) {
      result += `✅ Текст найден в Spicy Lyrics\n`;
      result += `   Тип: ${lyrics.type}\n`;
      if (lyrics.lines) {
        result += `   Строк: ${lyrics.lines.length}\n`;
      }
    } else {
      result += `⚠️ Запрос успешен, но текст не найден\n`;
      result += `   Это нормально - не все треки есть в базе\n`;
    }
    result += '\n✅ ВСЕ ПРОВЕРКИ ПРОЙДЕНЫ УСПЕШНО!';

  } catch (e) {
    result += `❌ ОШИБКА при запросе:\n`;
    result += `   ${e.toString()}\n\n`;

    if (e.toString().includes('401')) {
      result += `Решение: Токен истек, нажми "Подключить"\n`;
    } else if (e.toString().includes('Network')) {
      result += `Решение: Проверь интернет соединение\n`;
    } else if (e.toString().includes('Empty token')) {
      result += `Решение: Токен пустой, нажми "Подключить"\n`;
    }
  }

  console.log('[Diagnostic] Result:', result);
  alert(result);
}

async function importCsvData(csv, playlistName = "Импортированный плейлист") {
  const rows = parseSpotifyCsvRows(csv);
  if (!rows.length) {
    scToast('Импорт CSV', 'Не удалось распознать строки треков', false);
    setTimeout(scToastHide, 4000);
    return;
  }

  scToast('Импорт...', `Подготовка ${rows.length} треков`, true);

  const importedIds = [];
  let imported = 0;

  rows.forEach((row, index) => {
    const track = upsertSpotifyTrackFromCsvRow(row, index);
    if (!track) return;
    ensureTrackId(track);
    if (!importedIds.includes(track.id)) importedIds.push(track.id);
    imported++;
  });

  if (importedIds.length > 0) {
    const pId = 'p_' + Date.now() + Math.random().toString(36).substr(2, 5);
    P.push({
      id: pId,
      name: playlistName,
      trackIds: importedIds,
      locked: false,
      source: 'spotify'
    });
    savePlaylistsCache();
  }

  T.forEach((t, i) => { if (t._insertIdx == null) t._insertIdx = i; });
  rl();
  renderPlaylists();
  saveT();

  scToast('Импорт завершен', `Добавлено ${imported} треков из ${rows.length}`, false);
  setTimeout(scToastHide, 5000);
}

//  SPICY LYRICS ENGINE
const LX_CACHE_KEY_PREFIX = 'lx_cache_';
let currentLyrics = null;
Object.defineProperty(window, 'currentLyrics', { get: () => currentLyrics, set: (v) => currentLyrics = v });
let lastLyricsId = null;
let currentLyricsProvider = 'Unknown';
let lastMiniLyricsKey = '';

/**
 * Show a status/error message in the mini lyrics preview.
 * Hides lx-sec when msg is null/empty, shows it with the message otherwise.
 * This is the ONLY place that should write to lx-sec-b for status states,
 * so the placeholder and error text never overlap.
 */
function setLyricsStatus(msg) {
  window.setLyricsStatus = setLyricsStatus;
  const sec = document.getElementById('lx-sec');
  const body = document.getElementById('lx-sec-b');
  if (!sec || !body) return;
  if (!msg) {
    sec.style.display = 'none';
    if (body.__lqRoll) {
      body.__lqRoll('', '');
    } else {
      body.innerHTML = '';
    }
    return;
  }
  sec.style.display = 'block';
  if (body.__lqRoll) {
    body.__lqRoll(msg, '');
  } else {
    body.innerHTML = `<div class="mini-act" style="opacity:0.55;font-size:13px">${msg}</div>`;
  }
}

class Spring {
  constructor(stiffness = 120, damping = 14, mass = 1) {
    this.stiffness = stiffness;
    this.damping = damping;
    this.mass = mass;
    this.current = 0;
    this.target = 0;
    this.velocity = 0;
  }
  update(dt) {
    const force = -this.stiffness * (this.current - this.target) - this.damping * this.velocity;
    const acceleration = force / this.mass;
    this.velocity += acceleration * dt;
    this.current += this.velocity * dt;
    return this.current;
  }
}

const lxSprings = {
  scroll: new Spring(80, 16),
  scale: new Spring(150, 12),
};
lxSprings.scale.current = 1;
lxSprings.scale.target = 1;

function updateLyricsProviderUI() {
  const miniHdr = document.getElementById('lx-sec-h');
  const p = (currentLyricsProvider || 'Unknown').toUpperCase();
  if (miniHdr) miniHdr.textContent = `Lyrics • ${p}`;
  // Update provider buttons
  const btnSpicy = document.getElementById('lx-prov-spicy');
  const btnLrc   = document.getElementById('lx-prov-lrclib');
  if (btnSpicy) btnSpicy.classList.toggle('active', currentLyricsProvider === 'Spicy Lyrics');
  if (btnLrc)   btnLrc.classList.toggle('active',   currentLyricsProvider === 'LRCLIB');
}

// Force-fetch from a specific provider, ignoring cache
async function switchLyricsProvider(provider) {
  const track = T && T[cur];
  if (!track) return;

  // Show loading state on button
  const btnSpicy = document.getElementById('lx-prov-spicy');
  const btnLrc   = document.getElementById('lx-prov-lrclib');
  [btnSpicy, btnLrc].forEach(b => { if (b) b.disabled = true; });

  const inv = getTauriInvoke();
  const trackId = track.spotifyId || track.id;

  if (provider === 'LRCLIB') {
    try {
      if (!inv) throw new Error('No Tauri');
      const resBytes = await inv('fetch_lrclib', { trackName: track.title, artistName: track.artist });
      const data = JSON.parse(resBytes);
      if (data && data.length > 0) {
        const best = data[0];
        let lrc = null;
        if (best.syncedLyrics) {
          const lines = best.syncedLyrics.split('\n').map(l => {
            const m = l.match(/\[(\d+):(\d+\.\d+)\](.*)/);
            if (m) return { Type: 'Vocal', StartTime: parseInt(m[1]) * 60 + parseFloat(m[2]), Text: m[3].trim() };
            return null;
          }).filter(Boolean);
          if (lines.length > 0) lrc = { Type: 'Line', Content: lines, _provider: 'LRCLIB' };
        } else if (best.plainLyrics) {
          lrc = { Type: 'Static', Lines: best.plainLyrics.split('\n').map(t => ({ Text: t })), _provider: 'LRCLIB' };
        }
        if (lrc) {
          currentLyrics = lrc;
          currentLyricsProvider = 'LRCLIB';
          saveAsset(LX_CACHE_KEY_PREFIX + trackId, lrc).catch(() => {});
          updateLyricsProviderUI();
          renderLyrics();
          [btnSpicy, btnLrc].forEach(b => { if (b) b.disabled = false; });
          return;
        }
      }
    } catch (e) { console.warn('[Provider Switch] LRCLIB failed:', e); }
    // No lyrics found — show message but keep overlay open
    document.getElementById('lx-ov-list').innerHTML = '<div style="color:rgba(255,255,255,0.5);padding:40px;text-align:center;font-size:16px">LRCLIB не нашёл текст для этого трека</div>';
    currentLyricsProvider = 'LRCLIB';
    updateLyricsProviderUI();
    [btnSpicy, btnLrc].forEach(b => { if (b) b.disabled = false; });
    return;
  }

  if (provider === 'Spicy Lyrics') {
    if (!spotifyToken || spotifyToken.length < 10) {
      document.getElementById('lx-ov-list').innerHTML = '<div style="color:rgba(255,255,255,0.5);padding:40px;text-align:center;font-size:16px">Нет Spotify токена для Spicy Lyrics</div>';
      currentLyricsProvider = 'Spicy Lyrics';
      updateLyricsProviderUI();
      [btnSpicy, btnLrc].forEach(b => { if (b) b.disabled = false; });
      return;
    }
    const spotifyId = track.spotifyId;
    if (!spotifyId) {
      document.getElementById('lx-ov-list').innerHTML = '<div style="color:rgba(255,255,255,0.5);padding:40px;text-align:center;font-size:16px">Нет Spotify ID для этого трека</div>';
      currentLyricsProvider = 'Spicy Lyrics';
      updateLyricsProviderUI();
      [btnSpicy, btnLrc].forEach(b => { if (b) b.disabled = false; });
      return;
    }
    try {
      if (!inv) throw new Error('No Tauri');
      const raw = await inv('fetch_spicy_lyrics', { spotifyId, token: spotifyToken });
      const parsed = JSON.parse(raw);
      const queryItems = Array.isArray(parsed?.queries) ? parsed.queries : [];
      const lyricsJob = queryItems.find(q => q?.operation === 'lyrics' && q?.result?.data) || queryItems.find(q => q?.result?.data) || null;
      const rawLyrics = lyricsJob?.result?.data || null;
      if (rawLyrics) {
        // Normalize type
        const tl = (rawLyrics.Type || '').toLowerCase();
        if (tl === 'syllable') rawLyrics.Type = 'Syllable';
        else if (tl === 'line') rawLyrics.Type = 'Line';
        else if (tl === 'static') rawLyrics.Type = 'Static';
        if (rawLyrics.Type === 'Line' && Array.isArray(rawLyrics.Content)) {
          const fv = rawLyrics.Content.find(c => c.Type === 'Vocal' || !c.Type);
          if (fv?.Lead?.Syllables?.length > 0) rawLyrics.Type = 'Syllable';
        }
        currentLyrics = rawLyrics;
        currentLyricsProvider = 'Spicy Lyrics';
        saveAsset(LX_CACHE_KEY_PREFIX + trackId, rawLyrics).catch(() => {});
        updateLyricsProviderUI();
        renderLyrics();
        [btnSpicy, btnLrc].forEach(b => { if (b) b.disabled = false; });
        return;
      }
    } catch (e) { console.warn('[Provider Switch] Spicy Lyrics failed:', e); }
    document.getElementById('lx-ov-list').innerHTML = '<div style="color:rgba(255,255,255,0.5);padding:40px;text-align:center;font-size:16px">Spicy Lyrics не нашёл текст для этого трека</div>';
    currentLyricsProvider = 'Spicy Lyrics';
    updateLyricsProviderUI();
    [btnSpicy, btnLrc].forEach(b => { if (b) b.disabled = false; });
  }
}
window.switchLyricsProvider = switchLyricsProvider;

async function fetchLyricsForTrack(track) {
  if (!track) {
    console.log('[Lyrics] Track is null or undefined');
    return;
  }

  const trackId = track.spotifyId || track.id;
  console.log('[Lyrics] Fetching for track:', { title: track.title, artist: track.artist, spotifyId: track.spotifyId, id: trackId });

  if (lastLyricsId === trackId) {
    console.log('[Lyrics] Already loading this track, skipping');
    return;
  }

  lastLyricsId = trackId;
  currentLyrics = null;
  currentLyricsProvider = 'Unknown';
  lastMiniLyricsKey = '';
  updateLyricsProviderUI();
  setLyricsStatus('Загрузка текста...');
  document.getElementById('lx-ov-list').innerHTML = '';

  // Check Cache (IndexedDB)
  try {
    const cachedLyrics = await getAsset(LX_CACHE_KEY_PREFIX + trackId);
    if (cachedLyrics) {
      // Normalize type in cached lyrics too
      const normalized = (() => {
        const l = cachedLyrics;
        if (!l.Type) return l;
        const tl = l.Type.toLowerCase();
        if (tl === 'syllable') l.Type = 'Syllable';
        else if (tl === 'line') l.Type = 'Line';
        else if (tl === 'static') l.Type = 'Static';
        if (l.Type === 'Line' && Array.isArray(l.Content)) {
          const fv = l.Content.find(c => c.Type === 'Vocal' || !c.Type);
          if (fv?.Lead?.Syllables?.length > 0) l.Type = 'Syllable';
        }
        return l;
      })();
      const cachedType = normalized.Type || normalized.type;
      if (cachedType && cachedType !== 'Static') {
        console.log('[Lyrics] Found synced lyrics in IndexedDB cache');
        currentLyrics = normalized;
        currentLyricsProvider = normalized._provider || 'Cache';
        updateLyricsProviderUI();
        renderLyrics();
        return;
      }
      console.log('[Lyrics] Cache has Static lyrics, refreshing from network');
    }
  } catch (e) { console.warn('[Lyrics] Cache read error:', e); }

  console.log('[Lyrics] Not in cache, fetching fresh...');

  let spotifyId = track.spotifyId;

  // LRCLIB Fallback Helper
  const tryLRCLibFallback = async () => {
    try {
      console.log('[Lyrics] Trying LRCLIB fallback for:', track.title, '-', track.artist);
      const inv = getTauriInvoke();
      if (!inv) throw new Error("No Tauri Invoke found.");

      const resBytes = await inv('fetch_lrclib', { trackName: track.title, artistName: track.artist });
      const data = JSON.parse(resBytes);
      console.log('[Lyrics] LRCLIB response received, entries:', data?.length || 0);

      if (data && data.length > 0) {
        const best = data[0];
        if (best.syncedLyrics) {
          const lines = best.syncedLyrics.split('\n').map(l => {
            const m = l.match(/\[(\d+):(\d+\.\d+)\](.*)/);
            if (m) {
              return { StartTime: parseInt(m[1]) * 60 + parseFloat(m[2]), Text: m[3].trim() };
            }
            return null;
          }).filter(Boolean);
          if (lines.length > 0) {
            console.log('[Lyrics] LRCLIB synced lyrics loaded, lines:', lines.length);
            return { Type: 'Line', Content: lines, _provider: 'LRCLIB' };
          }
        } else if (best.plainLyrics) {
          console.log('[Lyrics] LRCLIB plain lyrics loaded');
          return { Type: 'Static', Lines: best.plainLyrics.split('\n').map(t => ({ Text: t })), _provider: 'LRCLIB' };
        }
      }
      console.log('[Lyrics] LRCLIB fallback returned no results');
    } catch (e) {
      console.error('[Lyrics] LRCLib error:', e);
    }
    return null;
  };

  // If no spotifyId, attempt to search
  if (!spotifyId) {
    console.log('[Lyrics] No Spotify ID provided, checking Spotify token...');
    if (!spotifyToken) {
      console.log('[Lyrics] No Spotify token, trying LRCLIB only');
      const lrc = await tryLRCLibFallback();
      if (lrc) {
        currentLyrics = lrc;
        currentLyricsProvider = lrc._provider || 'LRCLIB';
        saveAsset(LX_CACHE_KEY_PREFIX + trackId, lrc).catch(() => {});
        updateLyricsProviderUI();
        renderLyrics();
        return;
      }
      setLyricsStatus('Войдите в Spotify или добавьте ID трека');
      document.getElementById('lx-ov-list').innerHTML = '<div style="color:red;padding:20px;text-align:center">Нет Spotify токена, а публичные базы текста (LRCLIB) этот трек не нашли.</div>';
      return;
    }
    try {
      console.log('[Lyrics] Searching Spotify for:', track.title, '-', track.artist);
      const query = encodeURIComponent(`${track.title} ${track.artist}`);
      const searchRes = await fetch(`https://api.spotify.com/v1/search?q=${query}&type=track&limit=1`, {
        headers: { 'Authorization': `Bearer ${spotifyToken}` }
      });
      console.log('[Lyrics] Spotify search response status:', searchRes.status);

      const searchData = await searchRes.json();
      if (searchData.tracks && searchData.tracks.items.length > 0) {
        spotifyId = searchData.tracks.items[0].id;
        console.log('[Lyrics] Found Spotify track ID:', spotifyId);
        track.spotifyId = spotifyId;
        saveT();
      } else {
        console.log('[Lyrics] Spotify search returned no results');
      }
    } catch (e) {
      console.error('[Lyrics] Spotify search failed:', e);
    }
  }

  if (!spotifyId) {
    console.log('[Lyrics] No Spotify ID available');
    setLyricsStatus('Текст не найден');
    return;
  }

  // Check token validity before calling backend
  if (!spotifyToken || spotifyToken.length < 10) {
    console.error('[Spicy Lyrics] Invalid Spotify token:', spotifyToken ? 'too short' : 'missing', 'length:', spotifyToken?.length);
    const lrc = await tryLRCLibFallback();
    if (lrc) {
      currentLyrics = lrc;
      currentLyricsProvider = lrc._provider || 'LRCLIB';
      saveAsset(LX_CACHE_KEY_PREFIX + trackId, lrc).catch(() => {});
      updateLyricsProviderUI();
      renderLyrics();
      return;
    }
    setLyricsStatus('Ошибка авторизации');
    document.getElementById('lx-ov-list').innerHTML = '<div style="color:red;padding:20px;text-align:center">Spotify токен недействителен. Переподключитесь.</div>';
    return;
  }

  try {
    const inv = getTauriInvoke();
    if (!inv) throw new Error("No Tauri Invoke found.");

    const normalizeLyricsType = (lyr) => {
      if (!lyr || !lyr.Type) return lyr;
      // Normalize to Title case: 'syllable' -> 'Syllable', 'line' -> 'Line', 'static' -> 'Static'
      const t = lyr.Type;
      if (typeof t === 'string') {
        const tl = t.toLowerCase();
        if (tl === 'syllable') lyr.Type = 'Syllable';
        else if (tl === 'line') lyr.Type = 'Line';
        else if (tl === 'static') lyr.Type = 'Static';
      }
      // Detect Syllable by structure when Type is wrong:
      // If Type=Line but Content items have Lead.Syllables -> it's actually Syllable
      if (lyr.Type === 'Line' && Array.isArray(lyr.Content)) {
        const firstVocal = lyr.Content.find(c => c.Type === 'Vocal' || !c.Type);
        if (firstVocal?.Lead?.Syllables?.length > 0) {
          console.log('[Lyrics] Auto-correcting Type: Line → Syllable (found Lead.Syllables in content)');
          lyr.Type = 'Syllable';
        }
      }
      // Detect Line by structure when Type is missing
      if (!lyr.Type && Array.isArray(lyr.Content)) {
        const firstVocal = lyr.Content.find(c => c.Type === 'Vocal' || !c.Type);
        if (firstVocal?.Lead?.Syllables?.length > 0) lyr.Type = 'Syllable';
        else if (firstVocal?.Text) lyr.Type = 'Line';
      }
      // Detect if Syllable times are in seconds instead of ms (Spicy Lyrics always uses ms, LRCLIB uses s)
      // A syllable StartTime > 10000 almost certainly means ms, < 10000 likely seconds
      if (lyr.Type === 'Syllable' && Array.isArray(lyr.Content)) {
        const firstVocal = lyr.Content.find(c => (c.Type === 'Vocal' || !c.Type) && c.Lead?.Syllables?.[0]);
        const firstSyl = firstVocal?.Lead?.Syllables?.[0];
        if (firstSyl && firstSyl.StartTime !== undefined && firstSyl.StartTime < 1000 && firstSyl.EndTime !== undefined) {
          // Times are in seconds, multiply by 1000 to convert to ms
          console.log('[Lyrics] Syllable times appear to be in seconds, converting to ms');
          lyr.Content.forEach(c => {
            if (c.Lead?.Syllables) c.Lead.Syllables.forEach(s => { s.StartTime = (s.StartTime || 0) * 1000; s.EndTime = (s.EndTime || 0) * 1000; });
            if (Array.isArray(c.Background)) c.Background.forEach(bg => { if (bg.Syllables) bg.Syllables.forEach(s => { s.StartTime = (s.StartTime || 0) * 1000; s.EndTime = (s.EndTime || 0) * 1000; }); });
          });
        }
      }
      return lyr;
    };

    const extractLyricsFromResponse = (raw) => {
      const data = JSON.parse(raw);
      const queryItems = Array.isArray(data?.queries) ? data.queries : [];
      const lyricsJob =
        queryItems.find(q => q?.operation === 'lyrics' && q?.result?.data) ||
        queryItems.find(q => q?.result?.data) ||
        null;
      const rawLyrics = lyricsJob?.result?.data || null;
      return {
        data,
        lyrics: rawLyrics ? normalizeLyricsType(rawLyrics) : null
      };
    };

    const fetchLyricsBySpotifyId = async (id) => {
      const raw = await inv('fetch_spicy_lyrics', { spotifyId: id, token: spotifyToken });
      const parsed = extractLyricsFromResponse(raw);
      return { raw, ...parsed };
    };

    console.log('[Spicy Lyrics] ═════════════════════════════════════════');
    console.log('[Spicy Lyrics] Starting fetch_spicy_lyrics call');
    console.log('[Spicy Lyrics] Spotify ID:', spotifyId);
    console.log('[Spicy Lyrics] Token length:', spotifyToken.length);
    console.log('[Spicy Lyrics] Track:', track.title, '-', track.artist);

    addDebugLog('═════════════════════════════════════════');
    addDebugLog(`Запрос текста для: ${track.title} - ${track.artist}`);
    addDebugLog(`Spotify ID: ${spotifyId}`);
    addDebugLog(`Токен: ${spotifyToken.length} символов`);

    const { raw: res, data, lyrics } = await fetchLyricsBySpotifyId(spotifyId);
    console.log('[Spicy Lyrics] Backend response received (length:', res.length, ')');
    addDebugLog(`✓ Ответ получен (${res.length} байт)`);
    console.log('[Spicy Lyrics] Response parsed, structure:', Object.keys(data));
    addDebugLog(`✓ JSON распарсен, ключи: ${Object.keys(data).join(', ')}`);
    const lType = lyrics?.Type || lyrics?.type || 'unknown';
    console.log('[Spicy Lyrics] Lyrics found:', !!lyrics, 'Type:', lType);
    addDebugLog(`Текст найден: ${!!lyrics ? 'да (' + lType + ')' : 'нет'}`);

    if (lyrics) {
      console.log('[Spicy Lyrics] SUCCESS - Lyrics loaded!');
      addDebugLog('✓✓✓ УСПЕШНО! Текст загружен!');
      let finalLyrics = lyrics;
      if (lyrics?.Type === 'Static' || lyrics?.type === 'Static') {
        // Some track IDs return only static text. Try nearby Spotify matches
        // and pick the first synced variant to restore karaoke animation.
        try {
          const query = encodeURIComponent(`${track.title} ${track.artist}`.trim());
          const searchRes = await fetch(`https://api.spotify.com/v1/search?q=${query}&type=track&limit=5`, {
            headers: { 'Authorization': `Bearer ${spotifyToken}` }
          });
          if (searchRes.ok) {
            const searchData = await searchRes.json();
            const candidates = (searchData?.tracks?.items || [])
              .map(item => item?.id)
              .filter(id => id && id !== spotifyId)
              .slice(0, 4);
            for (const candidateId of candidates) {
              const candidate = await fetchLyricsBySpotifyId(candidateId);
              if (candidate.lyrics && candidate.lyrics.Type && candidate.lyrics.Type !== 'Static') {
                addDebugLog(`✓ Найден synced вариант по соседнему Spotify ID: ${candidateId}`);
                finalLyrics = candidate.lyrics;
                track.spotifyId = candidateId;
                saveT();
                break;
              }
            }
          }
        } catch (e) {
          console.warn('[Spicy Lyrics] Alternate Spotify ID search failed:', e);
        }
        addDebugLog('Spicy вернул Static — пытаюсь получить синхронную версию из LRCLIB');
        const lrcSynced = await tryLRCLibFallback();
        if (lrcSynced && lrcSynced.Type !== 'Static') {
          finalLyrics = lrcSynced;
          addDebugLog('✓ LRCLIB дал синхронный текст, включаю анимацию');
        }
      }
      currentLyrics = finalLyrics;
      currentLyricsProvider = finalLyrics._provider || 'Spicy Lyrics';
      saveAsset(LX_CACHE_KEY_PREFIX + trackId, finalLyrics).catch(() => {});
      updateLyricsProviderUI();
      renderLyrics();
    } else {
      console.log('[Spicy Lyrics] No lyrics in response, trying LRCLIB fallback...');
      addDebugLog('Текста в Spicy Lyrics нет, пытаюсь LRCLIB...');
      const lrc = await tryLRCLibFallback();
      if (lrc) {
        console.log('[Spicy Lyrics] LRCLIB fallback succeeded');
        currentLyrics = lrc;
        currentLyricsProvider = lrc._provider || 'LRCLIB';
        saveAsset(LX_CACHE_KEY_PREFIX + trackId, lrc).catch(() => {});
        updateLyricsProviderUI();
        renderLyrics();
        return;
      }
      console.log('[Spicy Lyrics] No lyrics found in any database');
      setLyricsStatus('Текст для этого трека пока отсутствует');
      document.getElementById('lx-ov-list').innerHTML = '<div style="color:#aaa;padding:20px;text-align:center">Текст не найден ни в одной базе данных.</div>';
    }
  } catch (e) {
    console.error('[Spicy Lyrics] ═════════════════════════════════════════');
    console.error('[Spicy Lyrics] ERROR during fetch:', e);
    console.error('[Spicy Lyrics] Error type:', e.name);
    console.error('[Spicy Lyrics] Error message:', e.message);
    console.error('[Spicy Lyrics] Error toString:', e.toString());

    addDebugLog('═════════════════════════════════════════');
    addDebugLog('Ошибка при загрузке текста');
    addDebugLog(`Тип ошибки: ${e.name}`);
    addDebugLog(`Сообщение: ${e.message}`);
    addDebugLog(`Подробности: ${e.toString()}`);

    if (e.toString().includes('401') || e.toString().includes('Unauthorized')) {
      console.error('[Spicy Lyrics] Token expired or invalid - needs reauth');
      addDebugLog('! Токен истекла или недействителен');
      setLyricsStatus('Авторизация истекла');
      document.getElementById('lx-ov-list').innerHTML = '<div style="color:red;padding:20px;text-align:center">Spotify сессия истекла. Переподключитесь.</div>';
      return;
    }

    console.log('[Spicy Lyrics] Trying LRCLIB fallback due to error...');
    addDebugLog('Попытка загрузить текст из LRCLIB...');
    const lrc = await tryLRCLibFallback();
    if (lrc) {
      console.log('[Spicy Lyrics] LRCLIB fallback succeeded after error');
      addDebugLog('✓ Текст загружен из LRCLIB (резервный источник)');
      currentLyrics = lrc;
      currentLyricsProvider = lrc._provider || 'LRCLIB';
      cache[trackId] = lrc;
      localStorage.setItem(LX_CACHE_KEY, JSON.stringify(cache));
      updateLyricsProviderUI();
      renderLyrics();
      return;
    }
    console.error('[Spicy Lyrics] All sources exhausted, showing error to user');
    addDebugLog('✗ Не удалось загрузить текст из всех источников');
    showDebugLogs('Ошибка загрузки текста');
  }
}

// ── SpicyLyrics Renderer ──────────────────────────────────────────────────────

// Stores per-word/syllable metadata for animation
// ─────────────────────────────────────────────────────────────────────────────
// SpicyLyrics Engine
// All overlay animation runs in _slPhysicsTick (rAF, 60fps).
// timeupdate only updates the mini preview (4Hz is fine for that).
// ─────────────────────────────────────────────────────────────────────────────

let _slWordMeta = [];  // per-word/syllable descriptor objects
let _slDotMeta  = [];  // per-dot descriptor objects
let _slLineInfo = [];  // per-vocal-line: { el, startTime, endTime }
let _slActiveIdx = -1;
let _slUserScrollUntil = 0; // timestamp — don't auto-scroll until past this

// Spline helpers matching SpicyLyrics ScaleRange / YOffsetRange
const _SL_SCALE_CURVE  = [[0, 0.95], [0.7, 1.025], [1, 1.0]];
const _SL_YOFF_CURVE   = [[0, 0.01],  [0.9, -0.017], [1, 0]];

function _slEvalCurve(curve, t) {
  t = Math.max(0, Math.min(1, t));
  for (let i = 0; i < curve.length - 1; i++) {
    const [t0, v0] = curve[i], [t1, v1] = curve[i + 1];
    if (t <= t1) return v0 + (v1 - v0) * ((t - t0) / (t1 - t0));
  }
  return curve[curve.length - 1][1];
}

function renderLyrics() {
  if (!currentLyrics) return;
  const list = document.getElementById('lx-ov-list');
  list.innerHTML = '';
  _slWordMeta = [];
  _slDotMeta  = [];
  _slLineInfo = [];

  const inner = document.createElement('div');
  inner.className = 'sl-scroll-inner';
  inner.setAttribute('data-lyrics-type', currentLyrics.Type);
  list.appendChild(inner);

  const lType    = currentLyrics.Type; // already normalized by normalizeLyricsType()
  const isStatic   = lType === 'Static';
  const isSyllable = lType === 'Syllable';
  const isLine     = lType === 'Line';
  const rawLines   = isStatic ? (currentLyrics.Lines || []) : (currentLyrics.Content || []);

  // ── Helper: get first vocal line's start time ──────────────
  const getVocalStartTime = (line) => {
    if (isLine) return line.StartTime ?? 0;
    // Syllable: times are in ms
    return ((line.Lead?.Syllables?.[0]?.StartTime ?? line.Lead?.StartTime ?? 0)) / 1000;
  };

  // ── Opening interlude dots ──────────────────────────────────
  if (!isStatic && rawLines.length > 0) {
    const firstVocal = rawLines.find(l => !l.Type || l.Type === 'Vocal');
    if (firstVocal) {
      const firstStart = getVocalStartTime(firstVocal);
      if (firstStart > 3) _slAppendDots(inner, 0, firstStart);
    }
  }

  // ── We need a "lineInfoIdx" that maps DOM lines to _slLineInfo ─
  // rawLines contains both Vocal and non-Vocal entries. _slLineInfo
  // only stores Vocal lines. We use a stable DOM id based on rawLines idx,
  // but the lineIdx stored in _slWordMeta must match the index in _slLineInfo.
  // Solution: use _slLineInfo.length at push time as the lineInfoIdx.

  rawLines.forEach((line, rawIdx) => {
    // ── Static ──────────────────────────────────────────────────
    if (isStatic) {
      const el = document.createElement('div');
      el.className = 'l-line static-line';
      el.id = `lx-l-${rawIdx}`;
      _slBuildWords(el, (line.Text || '').split(' '), []);
      inner.appendChild(el);
      return;
    }

    // Skip non-Vocal entries (e.g. interlude markers with no text)
    if (line.Type && line.Type !== 'Vocal') return;

    // ── Line sync ────────────────────────────────────────────────
    if (isLine) {
      if (!line.Text) return;
      const lineInfoIdx = _slLineInfo.length; // index this line will get in _slLineInfo

      const el = document.createElement('div');
      const isOpposite = line.OppositeAligned === true;
      el.className = 'l-line' + (isOpposite ? ' opposite-line' : '');
      el.id = `lx-l-${rawIdx}`;

      const words = line.Text.split(' ').filter(Boolean);
      // Find next vocal line for timing
      let nextVocalStartTime = Infinity;
      for (let ni = rawIdx + 1; ni < rawLines.length; ni++) {
        const nl = rawLines[ni];
        if (!nl.Type || nl.Type === 'Vocal') {
          nextVocalStartTime = nl.StartTime ?? Infinity;
          break;
        }
      }
      const lineStartT = line.StartTime ?? 0;
      const lineEndT   = line.EndTime ?? nextVocalStartTime;
      const dur = Math.max(0.001, lineEndT - lineStartT);

      words.forEach((w, wi) => {
        const span = document.createElement('span');
        span.className = 'l-word' + (wi === words.length - 1 ? ' last-word' : '');
        span.textContent = w;
        const wStart = lineStartT + dur * (wi / words.length);
        const wEnd   = lineStartT + dur * ((wi + 1) / words.length);
        _slWordMeta.push({
          el: span, lineIdx: lineInfoIdx, startTime: wStart, endTime: wEnd, type: 'line-word',
          scaleSpring: _slMakeSpring(1, 40, 10),
          ySpring:     _slMakeSpring(0, 45, 11),
          _prevGP: '-20%'
        });
        el.appendChild(span);
      });

      el.onclick = () => { if (aud) aud.currentTime = lineStartT; };
      inner.appendChild(el);
      _slLineInfo.push({ el, startTime: lineStartT, endTime: nextVocalStartTime, idx: rawIdx });

      // Interlude between lines
      if (nextVocalStartTime !== Infinity) {
        const gap = nextVocalStartTime - (line.EndTime ?? lineStartT + 2);
        if (gap > 3) _slAppendDots(inner, line.EndTime ?? lineStartT + 2, nextVocalStartTime);
      }
      return;
    }

    // ── Syllable sync ────────────────────────────────────────────
    if (isSyllable) {
      const lead = line.Lead;
      if (!lead) return;
      const syls = lead.Syllables || [];
      if (!syls.length) return;

      const lineInfoIdx = _slLineInfo.length;

      // Times in ms → seconds
      const lineStart = (syls[0].StartTime ?? 0) / 1000;
      const lastSyl   = syls[syls.length - 1];
      const lineEnd   = ((lastSyl.EndTime ?? lastSyl.StartTime ?? 0)) / 1000;

      const isOpposite = lead.OppositeAligned === true;
      const el = document.createElement('div');
      el.className = 'l-line' + (isOpposite ? ' opposite-line' : '');
      el.id = `lx-l-${rawIdx}`;
      el.onclick = () => { if (aud) aud.currentTime = lineStart; };

      syls.forEach((s, si) => {
        const span = document.createElement('span');
        const isPartOf = s.IsPartOfWord === true;
        span.className = 'l-word' +
          (isPartOf ? ' part-of-word' : '') +
          (si === syls.length - 1 ? ' last-word' : '');
        span.textContent = s.Text;
        const sStart = (s.StartTime ?? 0) / 1000;
        const sEnd   = (s.EndTime   ?? ((s.StartTime ?? 0) + 500)) / 1000;
        _slWordMeta.push({
          el: span, lineIdx: lineInfoIdx, startTime: sStart, endTime: sEnd, type: 'syllable',
          scaleSpring: _slMakeSpring(0.95, 38, 9),
          ySpring:     _slMakeSpring(0.01, 48, 11),
          glowSpring:  _slMakeSpring(0, 28, 7),
          _prevGP: '-20%'
        });
        el.appendChild(span);
      });
      inner.appendChild(el);

      // Find next vocal for endTime of this line
      let nextVocalStart = Infinity;
      for (let ni = rawIdx + 1; ni < rawLines.length; ni++) {
        const nl = rawLines[ni];
        if (!nl.Type || nl.Type === 'Vocal') {
          const nlSyls = nl.Lead?.Syllables;
          nextVocalStart = nlSyls?.length
            ? (nlSyls[0].StartTime ?? 0) / 1000
            : (nl.Lead?.StartTime ?? 0) / 1000;
          break;
        }
      }
      _slLineInfo.push({ el, startTime: lineStart, endTime: nextVocalStart, idx: rawIdx });

      // ── Background vocal(s) ────────────────────────────────────
      const bgList = Array.isArray(line.Background) ? line.Background : [];
      bgList.forEach((bg, bgIdx) => {
        const bgSyls = bg.Syllables || [];
        if (!bgSyls.length) return;
        const bgLineInfoIdx = _slLineInfo.length;
        const bgStart = (bgSyls[0].StartTime ?? 0) / 1000;
        const bgLastSyl = bgSyls[bgSyls.length - 1];
        const bgEnd = ((bgLastSyl.EndTime ?? bgLastSyl.StartTime ?? 0)) / 1000;

        const bgEl = document.createElement('div');
        bgEl.className = 'l-line bg-line';
        bgEl.id = `lx-l-${rawIdx}-bg${bgIdx}`;
        bgSyls.forEach((s, si) => {
          const span = document.createElement('span');
          const isPartOf = s.IsPartOfWord === true;
          span.className = 'l-word' +
            (isPartOf ? ' part-of-word' : '') +
            (si === bgSyls.length - 1 ? ' last-word' : '');
          span.textContent = s.Text;
          const sStart = (s.StartTime ?? 0) / 1000;
          const sEnd   = (s.EndTime   ?? ((s.StartTime ?? 0) + 500)) / 1000;
          _slWordMeta.push({
            el: span, lineIdx: bgLineInfoIdx, startTime: sStart, endTime: sEnd, type: 'bg-syllable',
            scaleSpring: _slMakeSpring(0.95, 38, 9),
            ySpring:     _slMakeSpring(0.01, 48, 11),
            glowSpring:  _slMakeSpring(0, 28, 7),
            _prevGP: '-20%'
          });
          bgEl.appendChild(span);
        });
        bgEl.onclick = () => { if (aud) aud.currentTime = bgStart; };
        inner.appendChild(bgEl);
        // BG line gets its own entry in _slLineInfo so it animates independently
        _slLineInfo.push({ el: bgEl, startTime: bgStart, endTime: bgEnd + 1, idx: rawIdx * 1000 + bgIdx + 1, isBg: true });
      });

      // Interlude dots
      if (nextVocalStart !== Infinity && nextVocalStart - lineEnd > 3.5) {
        _slAppendDots(inner, lineEnd, nextVocalStart);
      }
    }
  });

  _slUpdateSongInfo();
  _slUpdateMiniPreview();
}

function _slMakeSpring(initVal, stiffness, damping) {
  const s = new Spring(stiffness, damping);
  s.current = initVal;
  s.target  = initVal;
  return s;
}

function _slBuildWords(el, words, meta) {
  words.filter(Boolean).forEach((w, wi) => {
    const span = document.createElement('span');
    span.className = 'l-word' + (wi === words.length - 1 ? ' last-word' : '');
    span.textContent = w;
    el.appendChild(span);
  });
}

function _slAppendDots(parent, startTime, endTime) {
  const dl = document.createElement('div');
  dl.className = 'sl-dot-line';
  dl.dataset.startTime = startTime;
  dl.dataset.endTime   = endTime;
  for (let i = 0; i < 3; i++) {
    const d = document.createElement('div');
    d.className = 'sl-dot';
    // Per-dot springs
    d.scaleSpring = _slMakeSpring(0.75, 42, 9);
    d.ySpring     = _slMakeSpring(0,    50, 11);
    d.opSpring    = _slMakeSpring(0.35, 28,  7);
    dl.appendChild(d);
    _slDotMeta.push({ el: d, parent: dl, startTime: startTime + (i * (endTime - startTime) / 3), endTime });
  }
  parent.appendChild(dl);
  return dl;
}

function _slUpdateSongInfo() {
  const infoEl  = document.getElementById('lx-ov-songinfo');
  const titleEl = document.getElementById('lx-ov-song-title');
  const artEl   = document.getElementById('lx-ov-song-artist');
  if (!infoEl || !titleEl || !artEl) return;
  const track = T && T[cur];
  if (track) {
    titleEl.textContent = track.title  || '';
    artEl.textContent   = track.artist || '';
    infoEl.style.display = 'block';
  }
}

function _slUpdateMiniPreview() {
  if (!currentLyrics) { setLyricsStatus(null); return; }
  const lxSec = document.getElementById('lx-sec');
  if (lxSec) lxSec.style.display = 'block';
  if (!aud) return;
  const time    = aud.currentTime;
  const isStatic = currentLyrics.Type === 'Static';
  const rawLines = isStatic ? (currentLyrics.Lines || []) : (currentLyrics.Content || []);

  // Find active line index in _slLineInfo (not rawLines)
  let activeLI = -1;
  if (!isStatic) {
    for (let i = 0; i < _slLineInfo.length; i++) {
      const li = _slLineInfo[i];
      if (!li.isBg && time >= li.startTime && time < li.endTime) { activeLI = i; break; }
    }
  }

  const preview = document.getElementById('lx-sec-b');
  if (!preview) return;

  if (activeLI !== -1) {
    // Get the rawIdx from _slLineInfo to look up the original line
    const rawIdx = _slLineInfo[activeLI].idx;
    const curr = rawLines[rawIdx];
    // Find next non-BG vocal line
    let nextRawIdx = -1;
    for (let i = activeLI + 1; i < _slLineInfo.length; i++) {
      if (!_slLineInfo[i].isBg) { nextRawIdx = _slLineInfo[i].idx; break; }
    }
    const next = nextRawIdx !== -1 ? rawLines[nextRawIdx] : null;

    const currTxt = curr ? (curr.Text || getSyllableText(curr)) : '';
    const nextTxt = next ? (next.Text || getSyllableText(next)) : '';
    const key = `${activeLI}|${currTxt}`;
    if (lastMiniLyricsKey !== key) {
      lastMiniLyricsKey = key;
      const old = [...preview.children];
      if (old.length) {
        old.forEach(e => e.classList.add('mini-ly-out'));
        setTimeout(() => {
          if (lastMiniLyricsKey === key) {
            preview.innerHTML = `<div class="mini-act">${currTxt}</div>`;
            if (nextTxt) preview.innerHTML += `<div class="mini-next">${nextTxt}</div>`;
          }
        }, 180);
      } else {
        preview.innerHTML = `<div class="mini-act">${currTxt}</div>`;
        if (nextTxt) preview.innerHTML += `<div class="mini-next">${nextTxt}</div>`;
      }
    }
  } else if (isStatic) {
    const key = `static|0`;
    if (lastMiniLyricsKey !== key) {
      lastMiniLyricsKey = key;
      preview.innerHTML = `<div class="mini-act">${rawLines[0]?.Text || 'Текст доступен'}</div>`;
    }
  }
}

// Kept for compatibility
function updateLyricsDisplay() { _slUpdateMiniPreview(); }

function getSyllableText(line) {
  if (!line?.Lead?.Syllables) return line?.Text || '';
  return line.Lead.Syllables.map(s => (s.IsPartOfWord ? '' : ' ') + s.Text).join('').trim();
}

// ── Open / Close ──────────────────────────────────────────────────────────────
function openFullLyrics() {
  if (!currentLyrics) return;
  const ov = document.getElementById('lx-ov');
  const bg = document.getElementById('lx-ov-bg');
  const list = document.getElementById('lx-ov-list');
  if (T && T[cur]) {
    bg.style.backgroundImage = `url(${T[cur].art})`;
    _slUpdateSongInfo();
  }
  updateLyricsProviderUI();
  // Attach user-scroll detector ONCE
  if (!list._slScrollBound) {
    list._slScrollBound = true;
    const pause = () => { _slUserScrollUntil = performance.now() + 2500; };
    list.addEventListener('touchstart', pause, { passive: true });
    list.addEventListener('wheel',      pause, { passive: true });
    list.addEventListener('scroll',     () => {
      // Only count scroll as user-initiated if NOT caused by our spring
      if (!_slAutoScrolling) _slUserScrollUntil = performance.now() + 2000;
    }, { passive: true });
  }
  ov.classList.add('show');
  _slStartPhysicsLoop();
}

function closeFullLyrics() {
  document.getElementById('lx-ov').classList.remove('show');
  _slStopPhysicsLoop();
}

// ── Physics Loop ──────────────────────────────────────────────────────────────
let _slRafId = null;
let _slLastFrameTime = 0;
let _slAutoScrolling = false;

function _slStartPhysicsLoop() {
  if (_slRafId) return;
  _slLastFrameTime = performance.now();
  const loop = (now) => {
    const dt = Math.min((now - _slLastFrameTime) / 1000, 0.05);
    _slLastFrameTime = now;
    _slPhysicsTick(dt);
    if (document.getElementById('lx-ov')?.classList.contains('show')) {
      _slRafId = requestAnimationFrame(loop);
    } else {
      _slRafId = null;
    }
  };
  _slRafId = requestAnimationFrame(loop);
}

function _slStopPhysicsLoop() {
  if (_slRafId) { cancelAnimationFrame(_slRafId); _slRafId = null; }
}

// ── Main per-frame tick (60fps) ───────────────────────────────────────────────
function _slPhysicsTick(dt) {
  if (!currentLyrics || !aud) return;
  const list = document.getElementById('lx-ov-list');
  if (!list) return;

  const time     = aud.currentTime;
  const isStatic = currentLyrics.Type === 'Static';

  // ── Find active line index in _slLineInfo ──────────────────
  // activeLI = index into _slLineInfo array (not rawLines idx)
  let activeLI = -1;
  if (!isStatic) {
    for (let i = 0; i < _slLineInfo.length; i++) {
      const li = _slLineInfo[i];
      if (time >= li.startTime && time < li.endTime) { activeLI = i; break; }
    }
  }
  _slActiveIdx = activeLI;

  // ── Line class states ──────────────────────────────────────
  if (!isStatic) {
    _slLineInfo.forEach((li, liIdx) => {
      const el = li.el;
      if (!el) return;
      if (li.isBg) {
        // BG lines animate independently: active when time is within their range
        const bgActive = time >= li.startTime && time < li.endTime;
        if (bgActive) {
          el.classList.add('act');
          el.classList.remove('sung');
        } else if (time >= li.endTime) {
          el.classList.add('sung');
          el.classList.remove('act');
        } else {
          el.classList.remove('act', 'sung');
        }
        return;
      }
      const isSung   = liIdx < activeLI;
      const isActive = liIdx === activeLI;
      if (isActive) {
        if (!el.classList.contains('act')) el.classList.add('act');
        el.classList.remove('sung');
      } else if (isSung) {
        if (!el.classList.contains('sung')) el.classList.add('sung');
        el.classList.remove('act');
      } else {
        el.classList.remove('act', 'sung');
      }
    });
  }

  // ── Per-word spring animation ──────────────────────────────
  _slWordMeta.forEach(m => {
    const liIdx    = m.lineIdx; // index in _slLineInfo
    const isSung   = liIdx < activeLI || (liIdx === activeLI && time >= m.endTime);
    const isActive = liIdx === activeLI && time >= m.startTime && time < m.endTime;
    const notSung  = !isSung && !isActive;

    const progress = isActive
      ? Math.max(0, Math.min(1, (time - m.startTime) / Math.max(0.001, m.endTime - m.startTime)))
      : (isSung ? 1 : 0);

    // Gradient position: -20% (not sung) → 100% (sung), sweeping while active
    const targetGP = notSung ? -20 : isSung ? 100 : -20 + 120 * progress;

    if (liIdx === activeLI) {
      const gp = `${targetGP.toFixed(2)}%`;
      if (gp !== m._prevGP) {
        m.el.style.setProperty('--gradient-position', gp);
        m._prevGP = gp;
      }
    } else {
      if (!isSung && m._prevGP !== '-20%') {
        m.el.style.setProperty('--gradient-position', '-20%');
        m._prevGP = '-20%';
      } else if (isSung && m._prevGP !== '100%') {
        m.el.style.setProperty('--gradient-position', '100%');
        m._prevGP = '100%';
      }
    }

    // Spring-based scale + yOffset
    if (m.scaleSpring && m.ySpring) {
      const tScale = notSung ? 0.95 : isSung ? 1.0 : _slEvalCurve(_SL_SCALE_CURVE, progress);
      const tY     = notSung ? 0.01 : isSung ? 0.0 : _slEvalCurve(_SL_YOFF_CURVE,  progress);

      m.scaleSpring.target = tScale;
      m.ySpring.target     = tY;

      const cScale = m.scaleSpring.update(dt);
      const cY     = m.ySpring.update(dt);

      const wsStr = cScale.toFixed(4);
      const wyStr = cY.toFixed(5);
      if (m._ws !== wsStr) { m.el.style.setProperty('--ws', wsStr); m._ws = wsStr; }
      if (m._wy !== wyStr) { m.el.style.setProperty('--wy', wyStr); m._wy = wyStr; }
    }
  });

  // ── Dot spring animation ───────────────────────────────────
  _slDotMeta.forEach((dm, i) => {
    const isAct = time >= dm.startTime && time < dm.endTime;
    const isSungDot = time >= dm.endTime;

    const tScale = isAct ? 1.02 : isSungDot ? 1.0 : 0.75;
    const tY     = isAct ? -0.015 : 0;
    const tOp    = isAct ? 1.0 : isSungDot ? 0.65 : 0.35;

    dm.el.scaleSpring.target = tScale;
    dm.el.ySpring.target     = tY;
    dm.el.opSpring.target    = tOp;

    const cS = dm.el.scaleSpring.update(dt).toFixed(4);
    const cY = dm.el.ySpring.update(dt).toFixed(5);
    const cO = dm.el.opSpring.update(dt).toFixed(4);

    if (dm.el._ds !== cS) { dm.el.style.setProperty('--ds', cS); dm.el._ds = cS; }
    if (dm.el._dy !== cY) { dm.el.style.setProperty('--dy', cY); dm.el._dy = cY; }
    if (dm.el._do !== cO) { dm.el.style.setProperty('--do', cO); dm.el._do = cO; }

    if (dm.parent && i % 3 === 0) {
      dm.parent.classList.toggle('act', time >= dm.startTime && time < dm.endTime + (dm.endTime - dm.startTime));
    }
  });

  // ── Spring scroll (use li.el directly, not lx-l-${activeIdx}) ─
  if (activeLI !== -1) {
    const activeEl = _slLineInfo[activeLI]?.el;
    if (activeEl) {
      const targetTop = activeEl.offsetTop - (list.clientHeight / 2) + (activeEl.clientHeight / 2);
      lxSprings.scroll.target = targetTop;
    }
  }

  const now = performance.now();
  if (now > _slUserScrollUntil) {
    _slAutoScrolling = true;
    list.scrollTop = lxSprings.scroll.update(dt);
    _slAutoScrolling = false;
  } else {
    lxSprings.scroll.current = list.scrollTop;
    lxSprings.scroll.velocity = 0;
  }
}

// Legacy no-op
function updateLyricsPhysics() {}

// ── Hook into play loop ────────────────────────────────────────────────────────
const originalUpUI = upUI;
upUI = function (t) {
  originalUpUI(t);
  const lxOv = document.getElementById('lx-ov');
  if (lxOv?.classList.contains('show') && !_slRafId) _slStartPhysicsLoop();
};

// Mini preview on timeupdate (4Hz is fine)
aud.addEventListener('timeupdate', _slUpdateMiniPreview);


const originalPlayCurrentTrackObject = playCurrentTrackObject;
playCurrentTrackObject = async function (t, epoch) {
  await originalPlayCurrentTrackObject(t, epoch);
  if (epoch !== undefined && epoch !== playEpoch) return;
  fetchLyricsForTrack(t);
};

async function clearExpiredSpotifyToken() {
  try {
    const inv = getTauriInvoke();
    if (!inv) return;

    console.log('[Spotify] Clearing expired token from backend...');
    await inv('clear_spotify_token');
    spotifyToken = null;
    localStorage.removeItem('spotify_token');
    console.log('[Spotify] Token cleared successfully');
    addDebugLog('✗ Токен истек и был очищен');
  } catch (e) {
    console.error('[Spotify] Error clearing token:', e);
    spotifyToken = null;
    localStorage.removeItem('spotify_token');
  }
}

async function openSpotifyManager() {
  console.log('[Spotify Manager] Opening manager, token:', spotifyToken ? 'exists' : 'missing');

  if (!spotifyToken) {
    console.log('[Spotify Manager] No token, starting login');
    loginSpotify();
    return;
  }

  document.getElementById('m-sp').style.display = 'flex';
  document.getElementById('m-sp-list').innerHTML = '<div class="s-loading">Загрузка плейлистов...</div>';

  try {
    console.log('[Spotify Manager] Fetching playlists with token length:', spotifyToken.length);
    const playlists = await fetchSpotifyPlaylists();
    console.log('[Spotify Manager] Playlists loaded, count:', playlists?.length || 0);
    renderSpotifyPlaylists(playlists);
  } catch (e) {
    console.error('[Spotify Manager] ERROR:', {
      message: e.message,
      status: e.status,
      toString: e.toString(),
      stack: e.stack
    });

    if (e.status === 401 || e.toString().includes('401')) {
      console.log('[Spotify Manager] Token expired (401), trying sp_dc refresh...');
      await clearExpiredSpotifyToken();
      const refreshed = await refreshSpotifyTokenFromSpDc();
      if (refreshed) {
        // Retry loading playlists with new token
        try {
          const playlists = await fetchSpotifyPlaylists();
          renderSpotifyPlaylists(playlists);
          return;
        } catch (_) {}
      }
      document.getElementById('m-sp-list').innerHTML = '<div class="emp"><p>Токен истек.<br><button onclick="loginSpotify()" style="margin-top:10px;padding:8px 16px;background:#1DB954;color:white;border:none;border-radius:4px;cursor:pointer">Переподключиться</button></p></div>';
    } else {
      const errorMsg = e.message || e.toString() || 'Неизвестная ошибка';
      console.error('[Spotify Manager] Showing error to user:', errorMsg);
      document.getElementById('m-sp-list').innerHTML = `<div class="emp"><p style="color:#ff6b6b;white-space:pre-wrap">ОШИБКА:<br>${errorMsg}</p></div>`;
    }
  }
}

async function loginSpotify() {
  const inv = getTauriInvoke();
  if (!inv) {
    alert('Tauri API недоступна');
    return;
  }

  try {
    console.log('[Spotify] Opening Exportify window for authentication...');
    addDebugLog('Открытие окна авторизации...');
    await inv('start_spotify_login');
  } catch (e) {
    console.error('[Spotify Auth] Error:', e);
    addDebugLog('✗ Ошибка открытия окна: ' + e.message);
  }
}

function closeSpotifyManager() {
  document.getElementById('m-sp').style.display = 'none';
}

async function fetchSpotifyPlaylists() {
  console.log('[fetchSpotifyPlaylists] Starting fetch with token length:', spotifyToken.length);

  try {
    const resp = await fetch('https://api.spotify.com/v1/me/playlists?limit=50', {
      headers: { 'Authorization': `Bearer ${spotifyToken}` }
    });

    console.log('[fetchSpotifyPlaylists] Response status:', resp.status);

    if (!resp.ok) {
      const text = await resp.text();
      console.error('[fetchSpotifyPlaylists] API Error response:', text);

      const err = new Error(`Spotify API HTTP ${resp.status}: ${text}`);
      err.status = resp.status;
      throw err;
    }

    const data = await resp.json();
    console.log('[fetchSpotifyPlaylists] Success, items:', data.items?.length || 0);
    return data.items || [];

  } catch (e) {
    console.error('[fetchSpotifyPlaylists] Caught error:', {
      message: e.message,
      stack: e.stack,
      toString: e.toString()
    });
    throw e;
  }
}

function renderSpotifyPlaylists(playlists) {
  const list = document.getElementById('m-sp-list');
  list.innerHTML = '';
  if (!playlists.length) {
    list.innerHTML = '<div class="emp"><p>Плейлисты не найдены</p></div>';
    return;
  }

  playlists.forEach(pl => {
    const el = document.createElement('div');
    el.className = 'sp-pl-item';
    const art = pl.images && pl.images.length ? pl.images[0].url : '';
    const isImported = P.some(x => x.spotifyId === pl.id);

    el.innerHTML = `
      <img src="${art || ''}" class="sp-pl-art" onerror="this.src=''">
      <div class="sp-pl-info">
        <div class="sp-pl-name">${esc(pl.name)}</div>
        <div class="sp-pl-meta">${pl.tracks.total} треков</div>
      </div>
      <button class="sp-pl-btn ${isImported ? 'sync' : 'imp'}" onclick="syncSpotifyPlaylist('${pl.id}', '${esc(pl.name).replace(/'/g, "\\'")}')">
        ${isImported ? 'Обновить' : 'Импорт'}
      </button>
    `;
    list.appendChild(el);
  });
}



async function syncSpotifyPlaylist(id, name) {
  const btn = event.currentTarget;
  btn.disabled = true;
  btn.textContent = '...';

  try {
    scToast('Spotify Sync', `Загрузка: ${name}`, true);
    const tracks = await fetchSpotifyPlaylistTracks(id);

    let plItem = P.find(x => x.spotifyId === id);
    if (!plItem) {
      plItem = createPlaylist(name, 'Импортировано из Spotify', { spotifyId: id, color: '167,139,250' });
    }

    let added = 0;
    tracks.forEach(t => {
      const track = t.track;
      if (!track) return;
      const spId = `sp_${track.id}`;
      const existing = T.find(x => x.id === spId);
      if (existing) {
        const es = track.id;
        if (es) {
          existing.spotifyUri = existing.spotifyUri || `spotify:track:${es}`;
          existing.spotifyWebUrl = existing.spotifyWebUrl || `https://open.spotify.com/track/${es}`;
        }
        if (!plItem.trackIds.includes(spId)) {
          plItem.trackIds.push(spId);
          added++;
        }
        return;
      }

      const sid = track.id;
      const newTrack = {
        id: spId,
        spotifyId: sid,
        spotifyUri: `spotify:track:${sid}`,
        spotifyWebUrl: `https://open.spotify.com/track/${sid}`,
        title: track.name,
        artist: track.artists.map(a => a.name).join(', '),
        album: track.album.name,
        art: track.album.images && track.album.images.length ? track.album.images[0].url : null,
        duration: track.duration_ms / 1000,
        liked: false,
        isSpotify: true
      };
      T.push(newTrack);
      plItem.trackIds.push(spId);
      added++;
    });

    if (plItem) {
      plItem.spotifyId = plItem.spotifyId || id;
      plItem.source = plItem.source || 'spotify';
      plItem.sourceUrl = plItem.sourceUrl || `https://open.spotify.com/playlist/${id}`;
    }

    savePlaylistsCache();
    saveT(); // Assuming saveT exists to persist tracks
    rl();
    if (typeof window.sbEnsurePlaylistUploaded === 'function' && plItem) {
      try {
        await window.sbEnsurePlaylistUploaded(plItem, { immediate: true, hydrateTracks: true });
      } catch (e) {
        console.warn('[SB-SYNC] Immediate Spotify playlist upload failed:', e);
      }
    }
    renderSpotifyPlaylists(await fetchSpotifyPlaylists()); // Refresh UI
    scToast('Spotify Sync', `Добавлено ${added} треков`, false);
    setTimeout(scToastHide, 3000);
  } catch (e) {
    console.error('[Spotify] Sync failed', e);
    scToast('Ошибка синхронизации', String(e), false);
    setTimeout(scToastHide, 3000);
  } finally {
    btn.disabled = false;
  }
}

async function fetchSpotifyPlaylistTracks(playlistId) {
  let tracks = [];
  let url = `https://api.spotify.com/v1/playlists/${playlistId}/tracks?limit=100`;

  while (url) {
    const resp = await fetch(url, {
      headers: { 'Authorization': `Bearer ${spotifyToken}` }
    });
    if (!resp.ok) throw new Error('Spotify API Tracks Error');
    const data = await resp.json();
    tracks = tracks.concat(data.items);
    url = data.next;
  }
  return tracks;
}

function saveT() {
  const persistableIds = new Set();
  P.forEach(pl => {
    if (!pl || pl.id === 'all' || pl._preview || !Array.isArray(pl.trackIds)) return;
    pl.trackIds.forEach(id => persistableIds.add(id));
  });
  const payload = T.filter(t => {
    if (!t) return false;
    if (!t._previewTransient) return true;
    ensureTrackId(t);
    return persistableIds.has(t.id) || isTrackReferencedBySavedPlaylist(t.id);
  });
  saveAppStateValue(TRACKS_CACHE_KEY, JSON.stringify(payload));
}

async function loadT() {
  const raw = await loadAppStateValue(TRACKS_CACHE_KEY);
  if (!raw) return;
  const parsed = JSON.parse(raw);
  if (!Array.isArray(parsed)) return;
  parsed.forEach(t => {
    if (T.some(x => x.id === t.id)) return;
    if (t.spotifyId) {
      t.spotifyUri = t.spotifyUri || `spotify:track:${t.spotifyId}`;
      t.spotifyWebUrl = t.spotifyWebUrl || `https://open.spotify.com/track/${t.spotifyId}`;
    }
    if (t.spotifyId && t.url && t.streamUrl && t.isSpotify) {
      t.isSpotify = false;
      t.isSoundCloud = true;
      if (!t.scPlaybackUrl) t.scPlaybackUrl = t.url;
    }
    T.push(t);
  });
}

// Initialize on load
(async () => {
  // Hide dropzone during load to avoid flash of empty state
  const dz = document.getElementById('dz');
  if (dz) dz.style.visibility = 'hidden';

  await loadSpotifyTokenFromStorage();  // Load token from Tauri Store first
  initSpotify();
  await initializeSpotifyStatus();
  await loadPlaylistsCache();
  await loadT();
  hydrateSoundCloudCache();

  // Stamp insertion order for stable new/old sorting
  T.forEach((t, i) => { if (t._insertIdx == null) t._insertIdx = i; });

  // Init offline mode (wait for it to mark cached tracks before rendering)
  await initOfflineMode();

  renderPlaylists();
  rl();

  // Restore last playing track (UI only, no auto-play)
  try {
    console.log('[startup] attempting to restore last track');
    await restoreLastTrack();
  } catch (err) {
    console.error('[startup] restoreLastTrack failed:', err);
  }
  rl();

  // Now show dropzone only if truly empty
  if (dz) dz.style.visibility = '';

  syncNativeNowPlayingNotification();
})();

// --- RECENT PLAYLISTS (CONTEXTS) TRACKER ---
(function() {
  const RECENT_KEY = 'liquify_recent_contexts_v2';
  let recentContextIds = [];

  function loadRecentContexts() {
    try {
      const raw = localStorage.getItem(RECENT_KEY);
      if (raw) recentContextIds = JSON.parse(raw);
    } catch(e) {}
    if (!Array.isArray(recentContextIds)) recentContextIds = [];
  }

  function saveRecentContexts() {
    localStorage.setItem(RECENT_KEY, JSON.stringify(recentContextIds));
  }

  window.pushRecentContext = function(plId) {
    if (!plId || plId === 'all') return;
    recentContextIds = recentContextIds.filter(id => id !== plId);
    recentContextIds.unshift(plId);
    if (recentContextIds.length > 6) recentContextIds.length = 6;
    saveRecentContexts();
    renderHomeRecentPlaylists();
  };

  function renderHomeRecentPlaylists() {
    const grid = document.querySelector('.hm-grid');
    if (!grid) return;
    
    if (typeof P === 'undefined' || !P || !P.length) return;

    const validRecents = recentContextIds.map(id => P.find(p => p.id === id)).filter(Boolean);
    const fallbacks = [
      P.find(p => p.id === 'favorites') || { id: 'favorites', name: 'Любимые треки' },
      P.find(p => p.id === 'recent') || { id: 'recent', name: 'Недавно играло' },
      { id: 'all', name: 'Все треки', fakeAction: "sv('l')" },
      { id: 'local', name: 'Моя медиатека', fakeAction: "sv('l')" },
      { id: 'search', name: 'Поиск треков', fakeAction: "sv('s')" },
      { id: 'playlists', name: 'Плейлисты', fakeAction: "sv('l')" }
    ];
    
    const displayList = [];
    const seen = new Set();
    
    for (const pl of validRecents) {
      if (!seen.has(pl.id) && pl.id !== 'all') {
        displayList.push(pl);
        seen.add(pl.id);
      }
    }
    for (const pl of fallbacks) {
      if (displayList.length >= 6) break;
      if (!seen.has(pl.id)) {
        displayList.push(pl);
        seen.add(pl.id);
      }
    }

    grid.innerHTML = '';
    
    const colors = [
      'linear-gradient(135deg,rgba(167,139,250,.6),rgba(80,40,140,.8))',
      'linear-gradient(135deg,rgba(245,158,11,.5),rgba(120,53,15,.8))',
      'linear-gradient(135deg,rgba(239,68,68,.5),rgba(100,10,10,.8))',
      'linear-gradient(135deg,rgba(16,185,129,.5),rgba(6,78,59,.8))',
      'linear-gradient(135deg,rgba(59,130,246,.5),rgba(30,58,138,.8))',
      'linear-gradient(135deg,rgba(236,72,153,.5),rgba(100,10,70,.8))'
    ];

    displayList.forEach((pl, i) => {
      const el = document.createElement('div');
      el.className = 'hm-card liquify-card';
      
      if (pl.fakeAction) {
        el.setAttribute('onclick', pl.fakeAction);
      } else {
        el.setAttribute('onclick', `openHomePlaylist('${pl.id}')`);
      }

      let artHtml = '';
      if (!pl.fakeAction && typeof renderPlaylistArtwork === 'function') {
        const innerHtml = renderPlaylistArtwork(pl, 26);
        if (innerHtml.includes('pl-art-fallback')) {
           artHtml = `<div class="hm-card-art" style="background:${colors[i%6]}">${innerHtml.replace(/width="[^"]+"/, 'width="26"').replace(/height="[^"]+"/, 'height="26"')}</div>`;
        } else {
           artHtml = `<div class="hm-card-art" style="padding:0">${innerHtml}</div>`;
        }
      } else {
        let iconSvg = '<svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,.9)" stroke-width="1.8" stroke-linecap="round"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/></svg>';
        if (pl.id === 'all') iconSvg = '<svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,.9)" stroke-width="1.8" stroke-linecap="round"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>';
        if (pl.id === 'local') iconSvg = '<svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,.9)" stroke-width="1.8" stroke-linecap="round"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5" fill="rgba(255,255,255,.7)" stroke="none"/><polyline points="21 15 16 10 5 21"/></svg>';
        if (pl.id === 'search') iconSvg = '<svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,.9)" stroke-width="1.8" stroke-linecap="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>';
        if (pl.id === 'playlists') iconSvg = '<svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,.9)" stroke-width="1.8" stroke-linecap="round"><line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/></svg>';
        artHtml = `<div class="hm-card-art" style="background:${colors[i%6]}">${iconSvg}</div>`;
      }

      el.innerHTML = `${artHtml}<span class="hm-card-name">${pl.name}</span>`;
      grid.appendChild(el);
    });
  }

  window.renderHomeRecentPlaylists = renderHomeRecentPlaylists;

  let _origUpUI = window.upUI;
  window.upUI = function(t) {
    if (typeof _origUpUI === 'function') _origUpUI(t);
    if (typeof activePlaylistId !== 'undefined' && activePlaylistId) {
      window.pushRecentContext(activePlaylistId);
    }
  };

  setTimeout(() => {
    loadRecentContexts();
    renderHomeRecentPlaylists();
  }, 500);
})();

// ═══════════════════════════════════════════════════════════════════════════════
// ─── SUPABASE SYNC LAYER ─────────────────────────────────────────────────────
// Handles all synchronization between the player and the Supabase database:
//   1. Track sync: every track played gets upserted to tracks + track_sources
//   2. Playlist sync: every playlist change syncs up to the DB
//   3. Now Playing: every play/pause/track change updates now_playing
//   4. Playlist restore: on login, playlists are loaded from the DB
// ═══════════════════════════════════════════════════════════════════════════════

(function() {
  'use strict';

  // ─── State ───────────────────────────────────────────────────────────────────

  let _sbSyncEnabled = false;
  let _sbAccessToken = null;
  let _sbUserId = null;
  let _sbNowPlayingTimer = null;
  let _sbSyncQueue = [];
  let _sbSyncRunning = false;
  let _sbTrackDbIdCache = new Map(); // localTrackId → dbUUID

  const SB_SYNC_DEBOUNCE_MS = 2000;
  const SB_NOW_PLAYING_INTERVAL_MS = 15000;

  // ─── Helpers ─────────────────────────────────────────────────────────────────

  function sbInvoke(cmd, args) {
    const inv = typeof getTauriInvoke === 'function'
      ? getTauriInvoke()
      : (typeof window.__TAURI__?.core?.invoke === 'function'
        ? window.__TAURI__.core.invoke
        : (typeof window.__TAURI__?.invoke === 'function'
          ? window.__TAURI__.invoke
          : (typeof window.__TAURI_INVOKE__ === 'function' ? window.__TAURI_INVOKE__ : null)));
    if (!inv) {
      console.warn('[SB-SYNC] Tauri invoke not available');
      return Promise.reject('no tauri invoke');
    }
    return inv(cmd, args);
  }

  function sbLog(msg) {
    console.log(`[SB-SYNC] ${msg}`);
  }

  function sbWarn(msg) {
    console.warn(`[SB-SYNC] ${msg}`);
  }

  function sbError(msg) {
    console.error(`[SB-SYNC] ${msg}`);
  }

  // ─── Initialize sync when user logs in ───────────────────────────────────────

  window.sbInitSync = function(session) {
    if (!session || !session.access_token || !session.user?.id) {
      sbWarn('sbInitSync called without valid session');
      return;
    }

    _sbAccessToken = session.access_token;
    _sbUserId = session.user.id;
    _sbSyncEnabled = true;

    sbLog(`Sync initialized for user ${_sbUserId}`);

    // Start now_playing periodic updater
    sbStartNowPlayingSync();

    // Load playlists from DB
    sbLoadPlaylistsFromDb();
  };

  window.sbDisableSync = function() {
    _sbSyncEnabled = false;
    _sbAccessToken = null;
    _sbUserId = null;

    if (_sbNowPlayingTimer) {
      clearInterval(_sbNowPlayingTimer);
      _sbNowPlayingTimer = null;
    }

    sbLog('Sync disabled');
  };

  function sbIsReady() {
    return _sbSyncEnabled && _sbAccessToken && _sbUserId;
  }

  async function sbEnsurePlaylistUploaded(pl, options) {
    if (!sbIsReady() || !pl || pl.id === 'all') return null;

    const opts = options || {};
    const source = pl.source || (pl.spotifyId ? 'spotify' : 'local');
    const sourceUrl = pl.sourceUrl || (pl.spotifyId ? `https://open.spotify.com/playlist/${pl.spotifyId}` : null);
    const existingDbTrackIds = [];

    for (const tid of (pl.trackIds || [])) {
      const dbId = _sbTrackDbIdCache.get(tid);
      if (dbId) existingDbTrackIds.push(dbId);
    }

    const playlistDbId = await sbInvoke('sync_playlist_to_db', {
      accessToken: _sbAccessToken,
      userId: _sbUserId,
      playlistId: pl._dbId || null,
      name: pl.name,
      description: pl.description || null,
      isPublic: false,
      trackIds: existingDbTrackIds,
      source: source,
      sourceUrl: sourceUrl || null,
    });

    if (playlistDbId) {
      pl._dbId = playlistDbId;
      sbLog(`Playlist shell uploaded: "${pl.name}" → ${playlistDbId} (${existingDbTrackIds.length} cached tracks)`);
    }

    if (opts.hydrateTracks === false) {
      return playlistDbId || null;
    }

    const hydratedDbTrackIds = [];
    for (const tid of (pl.trackIds || [])) {
      let dbId = _sbTrackDbIdCache.get(tid);
      if (!dbId) {
        const track = (window.T || []).find(t => t.id === tid);
        if (track) {
          try {
            await sbSyncTrack(track);
            dbId = _sbTrackDbIdCache.get(tid);
          } catch (e) {
            sbWarn(`Track hydration failed for playlist "${pl.name}": ${e}`);
          }
        }
      }
      if (dbId) hydratedDbTrackIds.push(dbId);
    }

    if ((playlistDbId || pl._dbId) && hydratedDbTrackIds.length !== existingDbTrackIds.length) {
      await sbInvoke('sync_playlist_to_db', {
        accessToken: _sbAccessToken,
        userId: _sbUserId,
        playlistId: pl._dbId || playlistDbId || null,
        name: pl.name,
        description: pl.description || null,
        isPublic: false,
        trackIds: hydratedDbTrackIds,
        source: source,
        sourceUrl: sourceUrl || null,
      });
      sbLog(`Playlist tracks hydrated: "${pl.name}" (${hydratedDbTrackIds.length} tracks)`);
    }

    return playlistDbId || pl._dbId || null;
  }

  // ─── Track Sync ──────────────────────────────────────────────────────────────
  // When a track is played, sync it to the DB with all its source links

  async function sbSyncTrack(track) {
    if (!sbIsReady() || !track) return;

    const localId = track.id || '';
    if (!localId) return;

    // Skip if already synced recently
    if (_sbTrackDbIdCache.has(localId)) return;

    try {
      const streamUrl = track.streamUrl || track.url || '';
      let provider = 'direct';
      let externalId = null;

      if (track.isSoundCloud) {
        provider = 'soundcloud';
        externalId = track.scTrackId || null;
      } else if (track.isSpotify) {
        provider = 'spotify';
        externalId = track.spotifyId || null;
      } else if (track.path) {
        provider = 'local';
        externalId = track.path;
      }

      // Determine artwork URL (skip data: URLs, they're too large)
      let artworkUrl = null;
      if (track.art && !track.art.startsWith('data:')) {
        artworkUrl = track.art;
      } else if (track.cover && !track.cover.startsWith('data:')) {
        artworkUrl = track.cover;
      }

      const durationMs = track.duration
        ? Math.round(track.duration * 1000)
        : null;

      const dbTrackId = await sbInvoke('sync_track_to_db', {
        accessToken: _sbAccessToken,
        title: track.title || 'Unknown',
        artist: track.artist || 'Unknown',
        album: track.album || null,
        durationMs: durationMs,
        artworkUrl: artworkUrl,
        streamUrl: streamUrl || null,
        provider: provider,
        externalId: externalId ? String(externalId) : null,
        spotifyId: track.spotifyId || null,
      });

      if (dbTrackId) {
        _sbTrackDbIdCache.set(localId, dbTrackId);
        track._dbId = dbTrackId;
        sbLog(`Track synced: "${track.title}" → ${dbTrackId}`);
      }
    } catch (e) {
      sbError(`Track sync failed: ${e}`);
    }
  }

  // ─── Now Playing Sync ────────────────────────────────────────────────────────

  async function sbUpdateNowPlaying() {
    if (!sbIsReady()) return;

    const aud = document.getElementById('aud');
    const t = (window.T && window.cur >= 0) ? window.T[window.cur] : null;

    try {
      const trackDbId = t ? (t._dbId || _sbTrackDbIdCache.get(t.id) || null) : null;
      const posMs = aud ? Math.round((aud.currentTime || 0) * 1000) : 0;
      const isPlaying = !!(aud && !aud.paused && t);

      await sbInvoke('update_now_playing_db', {
        accessToken: _sbAccessToken,
        userId: _sbUserId,
        trackId: trackDbId,
        positionMs: posMs,
        isPlaying: isPlaying,
      });
    } catch (e) {
      // Silently ignore — this fires frequently
    }
  }

  function sbStartNowPlayingSync() {
    if (_sbNowPlayingTimer) clearInterval(_sbNowPlayingTimer);
    _sbNowPlayingTimer = setInterval(sbUpdateNowPlaying, SB_NOW_PLAYING_INTERVAL_MS);
    // Also update immediately
    sbUpdateNowPlaying();
  }

  // ─── Playlist Sync ───────────────────────────────────────────────────────────

  let _sbPlaylistSyncDebounce = null;

  async function sbSyncAllPlaylists() {
    if (!sbIsReady()) return;

    const playlists = window.P || [];
    sbLog(`Syncing ${playlists.length} playlists to DB`);

    for (const pl of playlists) {
      // Skip the dynamic 'all' playlist — it's purely a UI concept
      if (pl.id === 'all') continue;

      try {
        // Resolve DB track IDs for the playlist tracks
        const dbTrackIds = [];
        for (const tid of (pl.trackIds || [])) {
          let dbId = _sbTrackDbIdCache.get(tid);
          if (!dbId) {
            // Try to find and sync the track
            const track = (window.T || []).find(t => t.id === tid);
            if (track) {
              await sbSyncTrack(track);
              dbId = _sbTrackDbIdCache.get(tid);
            }
          }
          if (dbId) dbTrackIds.push(dbId);
        }

        await sbInvoke('sync_playlist_to_db', {
          accessToken: _sbAccessToken,
          userId: _sbUserId,
          playlistId: pl._dbId || null,
          name: pl.name,
          description: pl.description || null,
          isPublic: false,
          trackIds: dbTrackIds,
          source: pl.source || 'local',
          sourceUrl: pl.sourceUrl || null,
        });

        sbLog(`Playlist synced: "${pl.name}" (${dbTrackIds.length} tracks)`);
      } catch (e) {
        sbError(`Playlist sync failed for "${pl.name}": ${e}`);
      }
    }
  }

  function sbSchedulePlaylistSync() {
    if (!sbIsReady()) return;
    if (_sbPlaylistSyncDebounce) clearTimeout(_sbPlaylistSyncDebounce);
    _sbPlaylistSyncDebounce = setTimeout(() => {
      sbSyncAllPlaylists().catch(e => sbError(`Playlist sync batch error: ${e}`));
    }, SB_SYNC_DEBOUNCE_MS);
  }

  // ─── Load playlists from DB on login ─────────────────────────────────────────

  async function sbLoadPlaylistsFromDb() {
    if (!sbIsReady()) return;

    try {
      sbLog('Loading playlists from DB...');
      const dbPlaylists = await sbInvoke('load_playlists_from_db', {
        accessToken: _sbAccessToken,
        userId: _sbUserId,
      });

      if (!Array.isArray(dbPlaylists) || dbPlaylists.length === 0) {
        sbLog('No playlists found in DB, syncing current state up...');
        // First login — push local playlists up
        sbSyncAllPlaylists();
        return;
      }

      sbLog(`Found ${dbPlaylists.length} playlists in DB`);

      // Merge DB playlists into local state
      for (const dbPl of dbPlaylists) {
        // Check if we already have this playlist locally
        const existing = (window.P || []).find(p =>
          p._dbId === dbPl.id || p.name === dbPl.name
        );

        if (existing) {
          // Link the DB ID
          existing._dbId = dbPl.id;
          sbLog(`Linked local playlist "${existing.name}" → DB ${dbPl.id}`);
        } else {
          // Create a new local playlist from DB data
          const newPl = {
            id: 'pl_db_' + dbPl.id.replace(/-/g, '').slice(0, 8),
            _dbId: dbPl.id,
            name: dbPl.name,
            description: dbPl.description || '',
            trackIds: [],
            source: 'synced',
            sourceUrl: '',
          };

          // Load track associations
          try {
            const dbPtracks = await sbInvoke('load_playlist_tracks_from_db', {
              accessToken: _sbAccessToken,
              playlistId: dbPl.id,
            });

            if (Array.isArray(dbPtracks) && dbPtracks.length > 0) {
              // Load actual track details and add to T[]
              for (const pt of dbPtracks) {
                try {
                  const dbTrack = await sbInvoke('load_track_from_db', {
                    accessToken: _sbAccessToken,
                    trackId: pt.track_id,
                  });

                  if (dbTrack) {
                    // Check if this track already exists in T[]
                    let localTrack = (window.T || []).find(t =>
                      t.title === dbTrack.title && t.artist === dbTrack.artist
                    );

                    if (!localTrack) {
                      // Load sources to get stream URL
                      const sources = await sbInvoke('load_track_sources_from_db', {
                        accessToken: _sbAccessToken,
                        trackId: pt.track_id,
                      });

                      const scSource = sources.find(s => s.provider === 'soundcloud');
                      const spotifySource = sources.find(s => s.provider === 'spotify');
                      const primarySource = sources[0];

                      localTrack = {
                        title: dbTrack.title,
                        artist: dbTrack.artist,
                        album: dbTrack.album || '',
                        duration: dbTrack.duration_ms ? dbTrack.duration_ms / 1000 : 0,
                        art: dbTrack.artwork_url || '',
                        cover: dbTrack.artwork_url || '',
                        url: primarySource?.stream_url || '',
                        streamUrl: primarySource?.stream_url || '',
                        isSoundCloud: !!scSource,
                        isSpotify: !scSource && !!spotifySource,
                        scTrackId: scSource?.external_id || '',
                        spotifyId: spotifySource?.external_id || '',
                        _dbId: pt.track_id,
                      };

                      if (typeof ensureTrackId === 'function') ensureTrackId(localTrack);
                      window.T.push(localTrack);
                    }

                    if (localTrack) {
                      if (typeof ensureTrackId === 'function') ensureTrackId(localTrack);
                      _sbTrackDbIdCache.set(localTrack.id, pt.track_id);
                      localTrack._dbId = pt.track_id;
                      if (!newPl.trackIds.includes(localTrack.id)) {
                        newPl.trackIds.push(localTrack.id);
                      }
                    }
                  }
                } catch (e) {
                  sbWarn(`Failed to load track ${pt.track_id}: ${e}`);
                }
              }
            }
          } catch (e) {
            sbWarn(`Failed to load playlist tracks for ${dbPl.id}: ${e}`);
          }

          window.P.push(newPl);
          sbLog(`Restored playlist from DB: "${newPl.name}" (${newPl.trackIds.length} tracks)`);
        }
      }

      // Re-save locally and re-render
      if (typeof savePlaylistsCache === 'function') savePlaylistsCache();
      if (typeof renderPlaylists === 'function') renderPlaylists();
      if (typeof rl === 'function') rl();

      sbLog('Playlist restore from DB complete');
    } catch (e) {
      sbError(`Failed to load playlists from DB: ${e}`);
    }
  }

  // ─── Hook into existing functions ────────────────────────────────────────────

  // 1. Hook play(): sync track + update now_playing on every track play
  const _origPlay = window.play;
  if (typeof _origPlay === 'function') {
    window.play = async function(i) {
      await _origPlay(i);
      if (sbIsReady() && window.T && window.T[i]) {
        const t = window.T[i];
        // Sync track in background (non-blocking)
        sbSyncTrack(t).catch(() => {});
        // Update now_playing immediately
        sbUpdateNowPlaying().catch(() => {});
      }
    };
  }

  // 2. Hook togglePlay(): update now_playing on pause/resume
  const _origTogglePlay = window.togglePlay;
  if (typeof _origTogglePlay === 'function') {
    window.togglePlay = function() {
      _origTogglePlay();
      if (sbIsReady()) {
        sbUpdateNowPlaying().catch(() => {});
      }
    };
  }

  // 3. Hook savePlaylistsCache(): trigger DB sync on any playlist change.
  // This must wrap the actual global function binding because playlist
  // mutations call savePlaylistsCache() directly, not window.saveAppStateValue().
  if (typeof savePlaylistsCache === 'function') {
    const _origFn = savePlaylistsCache;
    const wrappedSavePlaylistsCache = function(...args) {
      const result = _origFn.apply(this, args);
      if (sbIsReady()) {
        sbSchedulePlaylistSync();
      }
      return result;
    };
    window._sbOrigSavePlaylistsCache = _origFn;
    window.savePlaylistsCache = wrappedSavePlaylistsCache;
    try {
      savePlaylistsCache = wrappedSavePlaylistsCache;
    } catch (_) { }
  }

  // 4. Hook createPlaylist(): sync new playlists
  const _origCreatePlaylist = window.createPlaylist;
  if (typeof _origCreatePlaylist === 'function') {
    const wrappedCreatePlaylist = function(name, description, meta) {
      const pl = _origCreatePlaylist(name, description, meta);
      if (pl && sbIsReady()) {
        sbEnsurePlaylistUploaded(pl, { immediate: true, hydrateTracks: false }).catch(e => sbError(`Immediate playlist upload failed: ${e}`));
        sbSchedulePlaylistSync();
      }
      return pl;
    };
    window.createPlaylist = wrappedCreatePlaylist;
    try {
      createPlaylist = wrappedCreatePlaylist;
    } catch (_) { }
  }

  // 5. Hook deletePlaylist(): remove from DB too
  const _origDeletePlaylist = window.deletePlaylist;
  if (typeof _origDeletePlaylist === 'function') {
    window.deletePlaylist = function(plId) {
      _origDeletePlaylist(plId);
      if (sbIsReady()) {
        sbSchedulePlaylistSync();
      }
    };
  }

  // Expose for external use
  window.sbSyncTrack = sbSyncTrack;
  window.sbSyncAllPlaylists = sbSyncAllPlaylists;
  window.sbUpdateNowPlaying = sbUpdateNowPlaying;
  window.sbLoadPlaylistsFromDb = sbLoadPlaylistsFromDb;
  window.sbEnsurePlaylistUploaded = sbEnsurePlaylistUploaded;
  window.sbIsReady = sbIsReady;

  sbLog('Supabase sync layer loaded');
})();
