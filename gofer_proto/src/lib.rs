// We use a re-export pattern here to allow all the proto definitions to be under the same module when referencing.
// so instead of `use proto::gofer::{something}` it's just `use proto::{something}`.
#[allow(clippy::module_inception)]
mod proto;
pub use self::proto::*;
