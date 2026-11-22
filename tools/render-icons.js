// アイコンの SVG から各サイズの PNG と Windows 用 .ico を作る。
//   使い方: node tools/render-icons.js   （playwright と Chromium が必要）
// 出力: assets/icon/icon-<size>.png, assets/icon/icon.ico
const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const dir = path.join(__dirname, '..', 'assets', 'icon');
const main = fs.readFileSync(path.join(dir, 'icon.svg'), 'utf8');
const small = fs.readFileSync(path.join(dir, 'icon-small.svg'), 'utf8');
// 48px 以下は細部を省いた簡略版を使う
const svgFor = (s) => (s <= 48 ? small : main);
const SIZES = [16, 24, 32, 48, 64, 128, 180, 192, 256, 512, 1024];
const ICO_SIZES = [16, 24, 32, 48, 64, 128, 256];

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const png = {};
  for (const s of SIZES) {
    await page.setViewportSize({ width: s, height: s });
    const svg = svgFor(s).replace('width="1024" height="1024"', `width="${s}" height="${s}"`);
    await page.setContent(`<html><body style="margin:0;background:transparent">${svg}</body></html>`);
    png[s] = await page.screenshot({ omitBackground: true, clip: { x: 0, y: 0, width: s, height: s } });
    fs.writeFileSync(path.join(dir, `icon-${s}.png`), png[s]);
  }
  await browser.close();

  // ICO（PNG 埋め込み形式）
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(ICO_SIZES.length, 4);
  const entries = [];
  let offset = 6 + 16 * ICO_SIZES.length;
  for (const s of ICO_SIZES) {
    const e = Buffer.alloc(16);
    e.writeUInt8(s >= 256 ? 0 : s, 0);
    e.writeUInt8(s >= 256 ? 0 : s, 1);
    e.writeUInt8(0, 2);
    e.writeUInt8(0, 3);
    e.writeUInt16LE(1, 4);
    e.writeUInt16LE(32, 6);
    e.writeUInt32LE(png[s].length, 8);
    e.writeUInt32LE(offset, 12);
    offset += png[s].length;
    entries.push(e);
  }
  fs.writeFileSync(path.join(dir, 'icon.ico'), Buffer.concat([header, ...entries, ...ICO_SIZES.map((s) => png[s])]));
  console.log('ok:', SIZES.join(', '), '+ icon.ico');
})();
