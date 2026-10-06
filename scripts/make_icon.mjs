// 生成 EasyMD 占位图标（1024x1024 PNG），供 `tauri icon` 派生全套图标。
// 纯 Node 实现（zlib + 手写 PNG 编码），无第三方依赖。
// 设计：紫色圆角方块 (#8b6cef) + 白色 "E" 字形（Obsidian 风格品牌色）。
import { deflateSync } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";

const W = 1024;
const H = 1024;

const px = new Uint8Array(W * H * 4);

function setPx(x, y, r, g, b, a = 255) {
  if (x < 0 || y < 0 || x >= W || y >= H) return;
  const i = (y * W + x) * 4;
  px[i] = r;
  px[i + 1] = g;
  px[i + 2] = b;
  px[i + 3] = a;
}

// 圆角矩形内的判定（超采样 2x2 抗锯齿）
function inRoundedRect(sx, sy, m, r) {
  const x0 = m, y0 = m, x1 = W - m, y1 = H - m;
  if (sx < x0 || sx > x1 || sy < y0 || sy > y1) return false;
  const cx = Math.max(x0 + r, Math.min(sx, x1 - r));
  const cy = Math.max(y0 + r, Math.min(sy, y1 - r));
  const dx = sx - cx, dy = sy - cy;
  return dx * dx + dy * dy <= r * r;
}

// "E" 字形由一个竖条 + 三个横条构成
function inE(sx, sy) {
  const m = 288;                 // 左边距
  const wT = 96;                 // 笔画宽
  const wAll = W - m * 2;        // 总宽
  const hAll = H - m * 2;        // 总高
  // 竖条
  if (sx >= m && sx <= m + wT && sy >= m && sy <= m + hAll) return true;
  // 上/中/下横条（中条略短）
  const bars = [
    [m, wAll],
    [m + wT, wAll - wT - 56],
    [m, wAll],
  ];
  for (let i = 0; i < 3; i++) {
    const by = m + (hAll - wT) * (i / 2);
    if (sy >= by && sy <= by + wT && sx >= bars[i][0] && sx <= bars[i][0] + bars[i][1]) return true;
  }
  return false;
}

const PURPLE = [139, 108, 239];
const WHITE = [255, 255, 255];

for (let y = 0; y < H; y++) {
  for (let x = 0; x < W; x++) {
    let inside = 0;
    for (const [ox, oy] of [[0.25, 0.25], [0.75, 0.25], [0.25, 0.75], [0.75, 0.75]]) {
      if (inRoundedRect(x + ox, y + oy, 64, 224)) inside++;
    }
    if (inside > 0) {
      const [r, g, b] = inside >= 3 ? PURPLE : PURPLE;
      setPx(x, y, r, g, b, inside === 4 ? 255 : inside * 64);
      // E 字形（在方块内部再判断）
      let eHit = 0;
      for (const [ox, oy] of [[0.25, 0.25], [0.75, 0.25], [0.25, 0.75], [0.75, 0.75]]) {
        if (inE(x + ox, y + oy)) eHit++;
      }
      if (eHit >= 2) setPx(x, y, WHITE[0], WHITE[1], WHITE[2], 255);
    }
  }
}

// ---- PNG 编码 ----
function crc32(buf) {
  let table = crc32.table;
  if (!table) {
    table = crc32.table = new Int32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      table[n] = c;
    }
  }
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = table[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])));
  return Buffer.concat([len, typeBuf, data, crc]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(W, 0);
ihdr.writeUInt32BE(H, 4);
ihdr[8] = 8;  // bit depth
ihdr[9] = 6;  // RGBA
// 扫描线：每行前加 filter 字节 0
const raw = Buffer.alloc(H * (1 + W * 4));
for (let y = 0; y < H; y++) {
  raw[y * (1 + W * 4)] = 0;
  Buffer.from(px.buffer, y * W * 4, W * 4).copy(raw, y * (1 + W * 4) + 1);
}
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

const out = new URL("../src-tauri/app-icon.png", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, png);
console.log("icon written:", out);
