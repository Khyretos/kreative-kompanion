//! Linux and Windows machine stats (/proc and sysfs on Linux, Win32 on Windows), shared by the Kompanion server
//! (its own machine) and the runner (every other PC). No root needed.

pub mod fdinfo;
pub mod gpu;
pub mod paths;
pub mod system;
pub mod winsys;

pub use gpu::{GpuReader, GpuStats};
pub use system::{Sampler, Snapshot};
