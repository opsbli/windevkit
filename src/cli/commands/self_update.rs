use crate::self_update;

pub fn execute() -> anyhow::Result<()> {
    self_update::run()
}
