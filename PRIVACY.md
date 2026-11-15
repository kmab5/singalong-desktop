# Privacy

Singalong Desktop reads playback metadata and timing from Windows media sessions for enabled apps. It does not capture audio, use a microphone, identify music acoustically, create accounts, sync to a cloud service or send analytics.

When lyrics are needed and Offline mode is disabled, the app sends title/artist search information and available album metadata to LRCLIB and lyrics.ovh over HTTPS. Providers receive the connection IP address and handle requests under their own policies. No lyrics are sent to translation services in the first stable scope.

The library, lyrics, selections, timing adjustments, play counts, preferences and imported backup data stay in private local app storage. Backups and diagnostics are written only through explicit native file selection. Backups can contain personal library data and should be shared deliberately. Diagnostics are locally previewed and contain bounded application/feature state without songs, lyrics, player identities, listening records, user paths or credentials.

Closing the main window leaves tracking in the tray. Quit stops it. Starting at Windows sign-in is a separate opt-in, off by default. Offline mode cancels provider lookup and preserves local reading. Check for updates opens the public GitHub release page in the user's browser; there is no automatic updater.

Unsupported imported Android preferences and saved translations remain inert. They do not enable tracking, startup, network behavior or consent.
