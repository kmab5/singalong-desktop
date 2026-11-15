# Singalong interface standards

The desktop interface is an operating and reading surface. Preserve the established quiet dark palette, violet emphasis and Windows system typography. Lyrics take priority over application chrome. Do not add marketing decoration, invented artwork, gradients or new remote fonts.

Use semantic tokens from src/shared/styles.css: canvas, surface, muted text, accent, separator and control border. Main controls are at least 38px high; compact floating controls at least 30px. All controls, summaries and scroll regions have visible keyboard focus. Text/controls must remain readable at high zoom and on narrow windows. Body and metadata target 4.5:1 contrast; large lyric emphasis at least 3:1. State is always conveyed in text, not only color.

Main navigation stays separate from content. Settings use named sections rather than a continuous wall of controls. Consistent spacing groups related actions; destructive operations retain explicit confirmation. Blank, loading, offline and unavailable states must remain understandable.

Main and floating readers preserve independent appearance/follow state, original lyric text, uncertainty/timing labels and accessible complete text. Floating geometry and OS behavior require separate native verification. Synthetic headless rendering can check layout and interaction but cannot establish native window compatibility.

Respect reduced motion. User content wraps without page-wide overflow. Use local/system assets only and keep plan/ and docs/ outside app artifacts.
