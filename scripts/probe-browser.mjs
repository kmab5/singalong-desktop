// Native integration check with an isolated Chrome profile and original silent PCM.
// Does not visit a service, touch the user's browser profile or store listening data.
import { chromium } from '@playwright/test';
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { once } from 'node:events';

const pcm = Buffer.alloc(44 + 8000 * 2 * 120);
pcm.write('RIFF'); pcm.writeUInt32LE(pcm.length - 8, 4); pcm.write('WAVEfmt ', 8);
pcm.writeUInt32LE(16, 16); pcm.writeUInt16LE(1, 20); pcm.writeUInt16LE(1, 22);
pcm.writeUInt32LE(8000, 24); pcm.writeUInt32LE(16000, 28); pcm.writeUInt16LE(2, 32);
pcm.writeUInt16LE(16, 34); pcm.write('data', 36); pcm.writeUInt32LE(pcm.length - 44, 40);
const pageHtml = `<!doctype html><title>Singalong synthetic media probe</title>
<h1>Singalong synthetic media probe</h1><p>Silent PCM. No user media.</p>
<audio controls src="/silence.wav"></audio><button id="play">Play fixture</button><button id="pause">Pause fixture</button><button id="seek">Seek fixture</button>
<script>
const audio = document.querySelector('audio');
navigator.mediaSession.metadata = new MediaMetadata({title:'Singalong synthetic fixture'});
function anchor(){if(Number.isFinite(audio.duration)) navigator.mediaSession.setPositionState({duration:audio.duration,playbackRate:1,position:audio.currentTime});}
audio.onplay=()=>{navigator.mediaSession.playbackState='playing';anchor()};
audio.onpause=()=>{navigator.mediaSession.playbackState='paused';anchor()};
audio.onseeked=anchor;
document.querySelector('#play').onclick=()=>audio.play();
document.querySelector('#pause').onclick=()=>audio.pause();
document.querySelector('#seek').onclick=()=>{audio.currentTime=60};
</script>`;
const server = createServer((request, response) => {
  response.setHeader('Content-Type', request.url === '/silence.wav' ? 'audio/wav' : 'text/html');
  if (request.url === '/silence.wav') {
    response.setHeader('Accept-Ranges', 'bytes');
    const range = /^bytes=(\d+)-(\d*)$/.exec(request.headers.range ?? '');
    const start = range ? Number(range[1]) : 0;
    const end = range?.[2] ? Math.min(Number(range[2]), pcm.length - 1) : pcm.length - 1;
    if (start > end || start >= pcm.length) { response.writeHead(416); response.end(); return; }
    if (range) {
      response.statusCode = 206;
      response.setHeader('Content-Range', `bytes ${start}-${end}/${pcm.length}`);
    }
    response.setHeader('Content-Length', end - start + 1);
    response.end(pcm.subarray(start, end + 1));
  } else response.end(pageHtml);
}).listen(0, '127.0.0.1');
await once(server, 'listening');
let browser;
async function observe() {
  const child = spawn('src-tauri/target/debug/examples/media_probe.exe', ['3']);
  let output = '', errors = '';
  child.stdout.on('data', chunk => { output += chunk; });
  child.stderr.on('data', chunk => { errors += chunk; });
  const [exit] = await once(child, 'exit');
  if (exit !== 0 || errors) throw new Error('Native probe did not exit cleanly');
  return output.trim().split(/\r?\n/).map(line => JSON.parse(line));
}
const chromeSnapshots = events => events.filter(event => event.event === 'snapshot' && /chrome/i.test(event.player_id));
try {
  browser = await chromium.launch({channel:'chrome',headless:false});
  const page = await browser.newPage();
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  await page.locator('#play').click();
  const playing = await observe();
  if (!chromeSnapshots(playing).some(event => event.status.endsWith('(4)'))) throw new Error('Chrome playing session not exposed');
  await page.locator('#pause').click();
  await page.locator('#seek').click();
  await page.waitForFunction(() => Math.abs(document.querySelector('audio').currentTime - 60) < 1);
  const paused = await observe();
  const pausedSeek = chromeSnapshots(paused).some(event => event.status.endsWith('(5)') && event.position_ms >= 59000);
  if (!pausedSeek) console.log(JSON.stringify({ diagnostic: 'synthetic Chrome paused-seek mismatch',
    browserState: await page.locator('audio').evaluate(audio => ({ position: audio.currentTime, duration: audio.duration, paused: audio.paused })),
    native: chromeSnapshots(paused).map(event => ({status:event.status,position:event.position_ms,end:event.end_ms})) }));
  const report = {
    browser: await browser.version(),
    playing: true, pausedSeek,
    simultaneousSessions: Math.max(...playing.filter(event => event.event === 'session_count').map(event => event.count)),
    missingArtistObserved: chromeSnapshots(playing).some(event => event.metadata_present?.artist === false),
  };
  await browser.close(); browser = undefined;
  const after = await observe();
  report.sessionRemovedAfterExit = chromeSnapshots(after).length === 0;
  if (!report.sessionRemovedAfterExit) throw new Error('Chrome session remained after browser exit');
  console.log(JSON.stringify(report, null, 2));
} finally {
  await browser?.close();
  server.closeAllConnections();
  await new Promise(resolve => server.close(resolve));
}
