# 246-winsys-win32-calls

winsys.rs, Win32 FFI. The Coder (3 attempts) put `#[cfg(windows)] #[allow(dead_code)]` above the pure `busy_total` so it vanished on Linux, passed a UTF-16 path without the NUL terminator and as `*const i16` to GetDiskFreeSpaceExW, used `..Default::default()` on MEMORYSTATUSEX (no Default), called GetTickCount64 outside `unsafe`, and `.unwrap_or(1).into()` on a NonZero. My spec also said "total_free for the used share" (wrong: used = total - free). Give the 9B the exact signatures AND a 5-line skeleton for every Win32 call; say that wide strings need a trailing 0.
