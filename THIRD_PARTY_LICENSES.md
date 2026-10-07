# Third-Party Licenses

This inventory was derived from the resolved packages in rust/Cargo.lock and
the package metadata returned by:

~~~text
cargo metadata --manifest-path rust/Cargo.toml --format-version 1 --locked --offline
~~~

The inventory reflects the final two-crate workspace on 2026-09-30. License
expressions below are declarations from each package's metadata; consult the
linked upstream repository and packaged license files for the applicable full
terms. The two OVCA workspace crates are not third-party packages and are
omitted.

| Package | Version | Declared license | Upstream |
|---|---:|---|---|
| itoa | 1.0.18 | MIT OR Apache-2.0 | [repository](https://github.com/dtolnay/itoa) |
| memchr | 2.8.3 | Unlicense OR MIT | [repository](https://github.com/BurntSushi/memchr) |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | [repository](https://github.com/dtolnay/proc-macro2) |
| quote | 1.0.47 | MIT OR Apache-2.0 | [repository](https://github.com/dtolnay/quote) |
| serde | 1.0.229 | MIT OR Apache-2.0 | [repository](https://github.com/serde-rs/serde) |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | [repository](https://github.com/serde-rs/serde) |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | [repository](https://github.com/serde-rs/serde) |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | [repository](https://github.com/serde-rs/json) |
| syn | 2.0.119 | MIT OR Apache-2.0 | [repository](https://github.com/dtolnay/syn) |
| syn | 3.0.4 | MIT OR Apache-2.0 | [repository](https://github.com/dtolnay/syn) |
| thiserror | 1.0.69 | MIT OR Apache-2.0 | [repository](https://github.com/dtolnay/thiserror) |
| thiserror-impl | 1.0.69 | MIT OR Apache-2.0 | [repository](https://github.com/dtolnay/thiserror) |
| unicode-ident | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 | [repository](https://github.com/dtolnay/unicode-ident) |
| zmij | 1.0.23 | MIT | [repository](https://github.com/dtolnay/zmij) |
