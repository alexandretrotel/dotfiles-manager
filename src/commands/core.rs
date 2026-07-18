use crate::utils::display::{green, red};
use eyre::Result;

pub(crate) trait Command {
    fn name(&self) -> &str;
    fn execute(&mut self) -> Result<()>;
}

pub(crate) struct CommandExecutor;

impl CommandExecutor {
    pub(crate) fn run<T: Command>(task: &mut T) {
        let name = task.name().to_string();

        if let Err(e) = task.execute() {
            eprintln!("{}", red(&format!("Error during {}: {}", name, e)));
            return;
        }
        println!("{}", green(&format!("{} complete", name)));
    }
}
