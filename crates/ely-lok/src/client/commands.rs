use super::*;

impl Worker {
    /// Runs a command against the current selection.
    pub fn run(&mut self, doc: &Doc, id: &str, values: &[&str]) -> io::Result<()> {
        self.run_all(doc, &[(id, values)])
    }

    /// Runs in order; numbered arguments require their original revision.
    pub fn run_all(&mut self, doc: &Doc, commands: &[(&str, &[&str])]) -> io::Result<()> {
        self.run_commands(doc, commands, false)
    }

    /// Checks the revision before running the complete batch.
    pub fn run_all_at(&mut self, at: &Doc, commands: &[(&str, &[&str])]) -> io::Result<()> {
        self.run_commands(at, commands, true)
    }

    fn run_commands(&mut self, doc: &Doc, commands: &[(&str, &[&str])], checked: bool) -> io::Result<()> {
        let mut addressed = checked;
        let mut selection = false;
        for (id, values) in commands {
            let command = crate::commands::find(id)?;
            command.args_json(values)?;
            addressed |= command.addressed(values);
            selection |= command.target == crate::commands::Target::Selection;
        }
        if commands.is_empty() {
            return Ok(());
        }
        if addressed {
            let check = if checked && selection { tag::EXPECT_AT } else { tag::EXPECT };
            self.request(check, &Pack::new().u64(self.doc_id(doc)?).u64(doc.rev).bytes())?;
        }
        for (id, values) in commands {
            let pack = values.iter().fold(
                Pack::new().u64(self.doc_id(doc)?).str(id).u64(values.len() as u64),
                |pack, value| pack.str(value),
            );
            self.request(tag::POST_UNO, &pack.bytes())?;
        }
        Ok(())
    }
}
