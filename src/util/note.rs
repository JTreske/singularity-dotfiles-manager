use anyhow::Result;
use std::fmt::Display;

pub fn note(prompt: impl Display, msg: impl Display) -> Result<()> {
  Ok(cliclack::note(prompt, msg)?)
}
