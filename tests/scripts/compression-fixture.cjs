// Builds synthetic East Asian compression test text.
const fs = require('node:fs');
const path = require('node:path');
const d = require('docx');

async function main() {
  const output = process.argv[2];
  if (!output) throw new Error('Usage: compression-fixture.cjs output.docx');
  const text = '中文，标点。日本語、句読点。「かな」ぁぃぅぇぉ。';
  const doc = new d.Document({
    creator: 'Ely compatibility regression',
    styles: { default: { document: { run: { font: 'SimSun', size: 24 } } } },
    sections: [{
      properties: { page: { size: { width: 11906, height: 16838 },
        margin: { top: 1440, bottom: 1440, left: 1440, right: 1440 } } },
      children: [new d.Paragraph({ children: [new d.TextRun('COMPRESSION_MARKER')] }),
        ...Array.from({ length: 5 }, () => new d.Paragraph({ children: [new d.TextRun(text.repeat(6))] }))],
    }],
  });
  fs.mkdirSync(path.dirname(path.resolve(output)), { recursive: true });
  fs.writeFileSync(output, await d.Packer.toBuffer(doc));
}
main().catch(error => { console.error(error); process.exitCode = 1; });
