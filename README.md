# yclass-memflow

[Memflow] plugin for [YClass]. Also see: [ko1N/yclass-memflow](https://github.com/ko1N/yclass-memflow).

## How to use

1. [Configure YClass to use this plugin](https://github.com/dankope/yclass/blob/master/README.md#plugin-api)
2. Create `memflow-config.toml` in YClass config directory ([see YClass README](https://github.com/ItsEthra/yclass#plugin-api)).
3. Configure `memflow-config.toml` with desired settings.

### `memflow-config.toml`

```toml
# REQUIRED
# OS type (i.e. "win32", "native")
os = "win32"

# OPTIONAL
# Path to a directory with memflow plugins, if none then uses default scan locations.
scan_path = "./my_memflow_plugins/"
# Connector type (i.e. "kvm", "qemu", etc...)
conn = "kvm"
# Arguments to pass to the connector.
conn_args = "foo foo"
# Arguments to pass to the os.
os_args = "blah blah"
```

[Memflow]: https://github.com/memflow/memflow
[YClass]: https://github.com/ItsEthra/yclass
