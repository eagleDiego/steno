// @ts-check
import { invoke } from '@tauri-apps/api/core';

// DOM refs
const statusIndicator = document.getElementById('status-indicator');
const statusText = document.getElementById('status-text');
const btnStart = document.getElementById('btn-start');
const btnStop = document.getElementById('btn-stop');
const meetingName = document.getElementById('meeting-name');
const captureDuration = document.getElementById('capture-duration');
const captureStatus = document.getElementById('capture-status');

let durationInterval = null;
let captureStartTime = null;

// --- Tauri command wrappers ---

async function startCapture() {
  try {
    const result = await invoke('start_capture');
    if (result) {
      captureStartTime = Date.now();
      setStatus('recording', 'Recording');
      captureStatus.textContent = 'Recording';
      btnStart.disabled = true;
      btnStop.disabled = false;
      startDurationTimer();
    }
  } catch (err) {
    console.error('start_capture failed:', err);
    captureStatus.textContent = 'Error: ' + err;
  }
}

async function stopCapture() {
  try {
    await invoke('stop_capture');
    captureStartTime = null;
    setStatus('idle', 'Idle');
    captureStatus.textContent = 'Not running';
    btnStart.disabled = false;
    btnStop.disabled = true;
    stopDurationTimer();
  } catch (err) {
    console.error('stop_capture failed:', err);
  }
}

async function getStatus() {
  try {
    const status = await invoke('get_status');
    if (status) {
      captureStatus.textContent = status;
      if (status === 'recording' || status === 'detecting') {
        setStatus(status, status.charAt(0).toUpperCase() + status.slice(1));
        btnStart.disabled = true;
        btnStop.disabled = false;
      }
    }
  } catch (err) {
    // Silent fail on poll
  }
}

// --- UI helpers ---

function setStatus(mode, text) {
  statusIndicator.className = `status-${mode}`;
  statusText.textContent = text;
}

function startDurationTimer() {
  stopDurationTimer();
  durationInterval = setInterval(() => {
    if (!captureStartTime) return;
    const elapsed = Math.floor((Date.now() - captureStartTime) / 1000);
    const m = String(Math.floor(elapsed / 60)).padStart(2, '0');
    const s = String(elapsed % 60).padStart(2, '0');
    captureDuration.textContent = `${m}:${s}`;
  }, 1000);
}

function stopDurationTimer() {
  if (durationInterval) {
    clearInterval(durationInterval);
    durationInterval = null;
  }
}

// --- Event handlers ---

btnStart.addEventListener('click', startCapture);
btnStop.addEventListener('click', stopCapture);

// --- Init ---

// Poll status on load
getStatus();

// Every 5s poll status
setInterval(getStatus, 5000);