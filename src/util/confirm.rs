use anyhow::Result;
use std::fmt::Display;

pub fn confirm(msg: impl Display, initial_value: bool) -> Result<bool> {
  Ok(
    cliclack::confirm(msg)
      .initial_value(initial_value)
      .interact()?,
  )
}
