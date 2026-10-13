# 255: user id in the Windows logon task

The Coder (3 attempts) put `#[cfg(windows)]` on the pure `qualified_user` and on its test (so Linux never ran it), let an empty `%USERNAME%` through, wrote `env::var(..).ok().as_deref()` into a binding (borrow of a temporary), and added two junk lines (`user.pid()`) to `setup.rs`. Claude restored the test, removed the cfg, fixed the borrow and the empty name. Same family as 246/249: no `cfg` on logic that the Linux tests must run; bind Strings before `as_deref`.
