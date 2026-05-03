use anyhow::Result;
use std::fmt::Display;

pub fn password(msg: impl Display) -> Result<String> {
  // TODO: add validation
  Ok(cliclack::password(msg).mask('*').interact()?)
}
