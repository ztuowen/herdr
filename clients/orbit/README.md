# Orbit

Orbit is the experimental graphical Herdr client. This POC targets an already running default Herdr server and exposes a live diagnostics surface for the current JSON API.

Run the frontend and Tauri shell from this directory:

```bash
npm install
npm run tauri:dev
```

The bridge uses `HERDR_SOCKET_PATH` when present. Otherwise it reads the default local session socket from the same config location as a debug Herdr build.
