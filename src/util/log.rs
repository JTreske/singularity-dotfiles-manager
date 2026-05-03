use cliclack::log;
use std::fmt::Display;
use tracing::{error, info, warn};

pub fn step(msg: impl Display) {
  info!("{msg}");
  log::step(msg).unwrap();
}

pub fn info(msg: impl Display) {
  info!("{msg}");
  log::info(msg).unwrap();
}

pub fn success(msg: impl Display) {
  info!("{msg}");
  log::success(msg).unwrap();
}

pub fn remark(msg: impl Display) {
  info!("{msg}");
  log::remark(msg).unwrap();
}

pub fn warn(msg: impl Display) {
  warn!("{msg}");
  log::warning(msg).unwrap();
}

pub fn error(msg: impl Display) {
  error!("{msg}");
  log::error(msg).unwrap();
}
