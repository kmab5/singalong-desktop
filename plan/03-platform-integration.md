# Platform integration and feasibility gates

Status: proposed adapter strategy. No desktop OS/player combination was exercised during this planning task.

## Support strategy

Deliver the first native vertical slice on Windows, while keeping core code portable. Add Linux and macOS through the same observer contract. Windows-first delivery is confirmed; Linux/macOS integration is deferred beyond the first Windows stable release.

Expose capabilities per adapter/session: metadata, position, rate, artwork and observation availability. Missing position yields manual reading; missing duration disables duration-based estimates. Avoid presenting a universal supported-player list before testing.

The app remains an observer. Playback transport controls, audio decoding/playback, recognition and browser extensions are outside initial parity.

## Windows

Use Windows Runtime `Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager` through Rust Windows bindings. Observe session-list, metadata, playback and timeline changes; retain event registration tokens and remove them on shutdown. Inspect SourceAppUserModelId for native player identity and derive a friendly name without using the display name as a unique key.

Normalize WinRT TimeSpan values to milliseconds. Timeline Position is current as of LastUpdatedTime, which is UTC; anchor it carefully to local monotonic receipt time and bound stale values. Check start/end and seek ranges rather than assuming every EndTime is a valid song duration. The API documentation lists Windows 10 1809 introduction and globalMediaControl capability; P0 must verify actual access from the chosen unpackaged Tauri distribution rather than assuming a manifest declaration guarantees access. Sources: [session manager](https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessionmanager?view=winrt-26100), [timeline properties](https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessiontimelineproperties?view=winrt-26100).

Mandatory stable test targets are Windows 11 native Media Player, Spotify desktop, and YouTube separately in Chrome, Edge and Firefox. Also test Spotify web and Deezer web across those browsers, VLC and KMPlayer; document limitations. Other apps are best effort. These are release gates, not verified support claims. Test two concurrent players, missing metadata, paused seeks, rate changes, reused artwork and player exit. Unsupported players must leave the app useful for manual/imported reading.

A per-user application process and tray replace the Android foreground service. Do not install a Windows service. Test session lock/unlock and suspend/resume integration. Choose the advertised Windows OS floor only after Tauri/WebView2 and packaging checks; API availability alone is insufficient.

## Linux

Use the session D-Bus MPRIS interfaces: discover org.mpris.MediaPlayer2 names, watch bus owner changes, query metadata/playback/rate/position and respond to seek signals. Convert microseconds to milliseconds. Reconcile position periodically because event coverage varies by player. D-Bus name/instance and desktop identity are not the same; keep allowlisting separate from session ownership.

MPRIS standardizes discovery, querying and basic control for compliant players; it cannot establish compatibility with a player that does not expose it. See the [MPRIS specification](https://specifications.freedesktop.org/mpris/latest/).

Test at least one GNOME and one KDE setup, X11 and Wayland where available. Treat compositor handling of always-on-top, positioning, click-through, tray and fullscreen separately from playback compatibility. Do not promise identical overlay positioning on Wayland. If needed, ship an ordinary resizable mini-reader and report which window operations are unavailable.

Packaging sandbox portals, D-Bus access and selected-directory access must be tested in the intended package format, not only via cargo run. Broad session-bus access should not be granted as a shortcut.

## macOS

Make macOS a feasibility-gated adapter, not a clone of Windows code. Apple's public Now Playing material describes publishing an app's own playback; it does not establish a universal API for reading all other applications. See [Apple Now Playing](https://developer.apple.com/documentation/nowplaying). This is an evidence limitation, not proof that every alternative is impossible.

The later macOS feasibility phase should inspect installed players' public scripting dictionaries and evaluate app-specific Apple Events automation for Music and Spotify where supported. Verify required consent, sandbox/entitlement implications, metadata, position and rate fidelity on actual hardware. Invoke only fixed, trusted scripts with validated arguments; no general-purpose user script bridge.

Do not depend on private MediaRemote APIs or scrape UI as the default design. If no acceptable adapter meets the gate, ship the portable manual/library features and clearly list the limited tracked players or defer the macOS tracking release. Keep this decision recorded rather than advertising complete parity.

Test menu-bar tray behavior, Spaces/fullscreen behavior, focus, lock detection and signing/notarization with the actual chosen window settings.

## Floating window contract

Create an app-owned second webview using local assets; it shares core state and has narrower IPC permissions. Use a transparent, undecorated, always-on-top window where supported. Tauri exposes window APIs, but platform-specific behavior must be tested. See [Tauri webview window API](https://v2.tauri.app/reference/javascript/api/namespacewebviewwindow/) and [system tray](https://v2.tauri.app/learn/system-tray/).

Required behavior:

- Compact, expanded and minimal layouts with drag, resize and readable controls.
- Persistent geometry in logical units, associated with a monitor and clamped to its current work area.
- Recovery when a monitor disappears, scaling changes or saved bounds lie offscreen; tray includes Reset position.
- Manual hidden/minimized states never auto-open. Preserve 15-second pause and 8-second unavailable grace rules.
- A docked edge handle may reuse the same small window; test separately from true window minimize.
- Hide on session lock and suspend; restore only when allowed by prior user state. Never focus-steal on every song change.
- Throttle or suspend rendering when hidden. Collection remains controlled by tracking, not UI visibility.
- Explicit Quit tears down observers, jobs and windows. Close-to-tray is clearly communicated and recoverable.

Stable includes optional click-through, off by default, with a tray toggle that always restores interaction. Global shortcuts come later. Include Windows sign-in startup, off by default, launching into the tray and respecting previous tracking state. Auto-show lyrics is independently opt-in and never overrides explicit hide/minimize. Allow screen capture by default; capture exclusion is deferred.

## Network policy

Confirmed stable policy: allow lookup on any connection by default and provide an explicit Offline mode. Offline mode cancels provider requests and keeps cached lyrics available. Wi-Fi-only filtering is deferred. Report Android Wi-Fi-only preferences as unsupported/inert on import; never silently map them to a desktop setting or overwrite the current network choice.

## Selection and pin capability

Enable only user-selected apps. Automatic mode retains the current player through pauses and can switch after stop/disappearance. A pinned app/stream waits when unavailable; no fallback. App pins persist across launches. Stream pins require reliable identity to reconnect; ambiguity requires an explicit choice. If browser stream identity is inadequate, refuse a stream pin and offer a clearly labelled browser-app pin. A stream pin must never silently become browser-wide.

Validate stalled position with a refresh before freezing synchronization. Show Playback position unavailable and permit manual reading when verification fails. Do not confuse infrequent callbacks with a failed position source.

## P0 evidence format and gate

Record OS/build, architecture, desktop/compositor, app/package version, player version, session fields, callback cadence and pass/fail observations. Use synthetic or redacted metadata.

The Windows gate requires real metadata + position events, safe stop/restart, pause/seek correction, two-player selection, and an overlay that can be hidden/recovered without disrupting focus. Linux/macOS gates require equivalent evidence for their advertised scope. Failed gates trigger an updated support matrix and fallback, not silent substitution with audio capture or private APIs.
