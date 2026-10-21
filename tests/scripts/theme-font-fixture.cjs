// Builds text for supplemental theme-font regression.
const fs = require('node:fs');
const path = require('node:path');
const d = require('docx');

async function main() {
  const output = process.argv[2];
  if (!output) throw new Error('Usage: theme-font-fixture.cjs output.docx');
  const doc = new d.Document({
    creator: 'Ely compatibility regression',
    styles: { default: { document: { run: { font: 'Arial', size: 24 } } } },
    sections: [{
      properties: { page: { size: { width: 11906, height: 16838 },
        margin: { top: 1440, bottom: 1440, left: 1440, right: 1440 } } },
      children: ['THEME_FONT_MARKER', '中文主题字体：正文与标题。', '日本語：テーマと句読点。',
        '한국어 테마 글꼴', 'العربية والخطوط', 'ภาษาไทยและแบบอักษร']
        .map(text => new d.Paragraph({ children: [new d.TextRun(text)] })),
    }],
  });
  fs.mkdirSync(path.dirname(path.resolve(output)), { recursive: true });
  fs.writeFileSync(output, await d.Packer.toBuffer(doc));
}
main().catch(error => { console.error(error); process.exitCode = 1; });
