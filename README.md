[![Latest Version]][crates.io] [![Documentation]][docs.rs] ![License]

Set and get Unix file owner and group.

UID/GUI numbers or user/group names can be used.

Note: This crate will only compile on Unix systems.

```rust
use file_owner::PathExt;

"/tmp/baz".set_owner("nobody").unwrap();
"/tmp/baz".set_group("nogroup").unwrap();

let o = "/tmp/baz".owner().unwrap();
o.id(); // 99
o.name(); // Some("nobody")

let g = "/tmp/baz".group().unwrap();
g.id(); // 99
g.name(); // Some("nogroup")
```

See module level documentation on [docs.rs] for more examples.

[crates.io]: https://crates.io/crates/file-owner
[Latest Version]: https://img.shields.io/crates/v/file-owner.svg
[Documentation]: https://docs.rs/file-owner/badge.svg
[docs.rs]: https://docs.rs/file-owner
[License]: https://img.shields.io/crates/l/file-owner.svg
