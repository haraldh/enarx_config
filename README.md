This crate provides `EnarxConfig`, which can be used to with any `serde` deserializer.
Its main purpose is to read an `Enarx.toml` configuration file.

```rust
extern crate toml;
use enarx_config::EnarxConfig;
const CONFIG: &str = r#"
[[files]]
name = "LISTEN"
kind = "listen"
prot = "tls"
port = 12345
"#;

let config: EnarxConfig = toml::from_str(CONFIG).unwrap();
```
