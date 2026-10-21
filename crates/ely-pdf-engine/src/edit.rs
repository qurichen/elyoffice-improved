//! Page edits, undo and saving (PDF-02).

use std::{io, path::Path};

use ely_ipc::{Pack, Unpack};

use crate::client::{Doc, Worker};
use crate::proto::tag;

/// `ask` (the document) then sorted, distinct `pages`.
fn with_pages(ask: Pack, pages: &[usize]) -> Pack {
    pages
        .iter()
        .fold(ask.u64(pages.len() as u64), |ask, page| ask.u64(*page as u64))
}

fn utf8(path: &Path) -> io::Result<&str> {
    path.to_str()
        .ok_or_else(|| io::Error::other("a path that is not UTF-8"))
}

impl Worker {
    /// The history mark: equal marks, equal documents.
    pub fn history(&mut self, doc: Doc) -> io::Result<String> {
        self.mark(tag::HISTORY, self.whole(&doc)?)
    }

    fn mark(&mut self, kind: u8, ask: Pack) -> io::Result<String> {
        Unpack::new(&self.request(kind, &ask.bytes())?).str()
    }

    /// Undoes, or redoes, one edit.
    pub fn undo(&mut self, doc: Doc, redo: bool) -> io::Result<String> {
        self.mark(tag::UNDO, self.whole(&doc)?.u64(u64::from(redo)))
    }

    pub fn delete_pages(&mut self, doc: Doc, pages: &[usize]) -> io::Result<String> {
        self.mark(tag::DELETE, with_pages(self.on(&doc)?, pages))
    }

    /// Moves `pages` before page `to`, keeping their order.
    pub fn move_pages(&mut self, doc: Doc, pages: &[usize], to: usize) -> io::Result<String> {
        self.mark(tag::MOVE, with_pages(self.on(&doc)?, pages).u64(to as u64))
    }

    /// Turns `pages` clockwise by `by` degrees, a multiple of 90.
    pub fn rotate_pages(&mut self, doc: Doc, pages: &[usize], by: i32) -> io::Result<String> {
        self.mark(tag::ROTATE, with_pages(self.on(&doc)?, pages).i64(i64::from(by)))
    }

    /// Crops by margins as seen: top, right, bottom, left points.
    pub fn crop_pages(&mut self, doc: Doc, pages: &[usize], margins: [f32; 4]) -> io::Result<String> {
        let ask = margins
            .iter()
            .fold(with_pages(self.on(&doc)?, pages), |ask, side| ask.u64(u64::from(side.to_bits())));
        self.mark(tag::CROP, ask)
    }

    /// A blank page of `size` points at `at`.
    pub fn insert_blank(&mut self, doc: Doc, at: usize, (w, h): (f32, f32)) -> io::Result<String> {
        let ask = self.on(&doc)?.u64(at as u64);
        self.mark(tag::BLANK, ask.u64(u64::from(w.to_bits())).u64(u64::from(h.to_bits())))
    }

    /// Inserts every page of `path`; answers count and mark.
    pub fn insert_file(&mut self, doc: Doc, at: usize, path: &Path) -> io::Result<(usize, String)> {
        let ask = self.on(&doc)?.u64(at as u64).str(utf8(path)?).bytes();
        let reply = self.request(tag::INSERT, &ask)?;
        let mut reply = Unpack::new(&reply);
        Ok((reply.u64()? as usize, reply.str()?))
    }

    /// Writes only `pages` to `path`; the document stays.
    pub fn extract(&mut self, doc: Doc, pages: &[usize], path: &Path) -> io::Result<()> {
        let ask = with_pages(self.on(&doc)?, pages).str(utf8(path)?).bytes();
        self.request(tag::EXTRACT, &ask).map(|_| ())
    }

    /// Writes the document, compacted, to `path`.
    pub fn save(&mut self, doc: Doc, path: &Path) -> io::Result<()> {
        let ask = self.whole(&doc)?.str(utf8(path)?).bytes();
        self.request(tag::SAVE, &ask).map(|_| ())
    }
}
