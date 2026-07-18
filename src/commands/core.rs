use crate::utils::display::green;
use eyre::{Result, WrapErr};

pub(crate) trait Command {
    fn name(&self) -> &str;
    fn execute(&mut self) -> Result<()>;
}

pub(crate) struct CommandExecutor;

impl CommandExecutor {
    pub(crate) fn run<T: Command>(task: &mut T) -> Result<()> {
        let name = task.name().to_string();
        task.execute().wrap_err_with(|| format!("{name} failed"))?;
        println!("{}", green(&format!("{} complete", name)));
        Ok(())
    }
}
