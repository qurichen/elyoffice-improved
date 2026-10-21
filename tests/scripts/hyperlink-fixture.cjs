// Builds synthetic hyperlink colors without user documents.
const fs = require('node:fs');
const path = require('node:path');
const PptxGenJS = require('pptxgenjs');

async function main() {
  const output = process.argv[2];
  if (!output) throw new Error('Usage: hyperlink-fixture.cjs output.pptx');
  const ppt = new PptxGenJS();
  ppt.layout = 'LAYOUT_WIDE';
  ppt.author = 'Ely compatibility regression';
  ppt.subject = 'Synthetic hyperlink colors';
  ppt.title = 'Hyperlink colors';
  const slide = ppt.addSlide();
  const cases = [
    ['KEEP_BLACK', '000000'],
    ['KEEP_RED', 'CC0000'],
    ['THEME_BLUE', '000000'],
    ['HLINK_MODE', 'CC0000'],
    ['UNKNOWN_EXTENSION', 'CC0000'],
  ];
  cases.forEach(([text, color], index) => slide.addText(text, {
    x: 1, y: 0.6 + index * 1.2, w: 11, h: 0.8,
    fontFace: 'Arial', fontSize: 24, color,
    hyperlink: { url: 'https://example.com/' + text },
  }));
  fs.mkdirSync(path.dirname(path.resolve(output)), { recursive: true });
  await ppt.writeFile({ fileName: output });
}
main().catch(error => { console.error(error); process.exitCode = 1; });
