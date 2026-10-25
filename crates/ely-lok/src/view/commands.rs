use super::*;

impl LokView {
    /// Queues a command for the current selection.
    pub fn run(&mut self, id: &str, values: &[&str]) -> io::Result<()> {
        let command = crate::commands::find(id)?;
        if command.addressed(values) {
            return Err(io::Error::other(format!("{id} names something by number; run it at the state it was read")));
        }
        self.send(input::Input::Run { commands: vec![command.owned(values)?], at: None });
        Ok(())
    }

    /// Queues a command for the original read.
    pub fn run_at(&mut self, at: &Doc, id: &str, values: &[&str]) -> io::Result<()> {
        self.run_all_at(at, &[(id, values)])
    }

    /// Validates the source and queues one checked batch.
    pub fn run_all_at(&mut self, at: &Doc, commands: &[(&str, &[&str])]) -> io::Result<()> {
        if !self.doc.same_document(at) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "the read belongs to another document"));
        }
        let commands = commands.iter().map(|(id, values)| crate::commands::find(id)?.owned(values)).collect::<io::Result<_>>()?;
        self.send(input::Input::Run { commands, at: Some(at.clone()) });
        Ok(())
    }
}
