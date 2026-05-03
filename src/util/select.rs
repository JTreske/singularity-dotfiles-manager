use anyhow::Result;
use std::fmt::Display;

pub fn select<T>(
  msg: impl Display,
  initial_value: T,
  items: &[(T, impl Display, impl Display)],
) -> Result<T>
where
  T: Clone + Eq,
{
  Ok(
    cliclack::select(msg)
      .initial_value(initial_value)
      .items(items)
      .interact()?,
  )
}

pub fn multi_select<T>(
  msg: impl Display,
  initial_values: Vec<T>,
  items: &[(T, impl Display, impl Display)],
) -> Result<Vec<T>>
where
  T: Clone + Eq,
{
  Ok(
    cliclack::multiselect(msg)
      .initial_values(initial_values)
      .items(items)
      .max_rows(10)
      .interact()?,
  )
}
