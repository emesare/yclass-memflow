use std::{cell::RefCell, fs::File, io::Read, path::PathBuf};

use memflow::{
    prelude::{
        ConnectorArgs, IntoProcessInstanceArcBox, Inventory, MemoryView, ModuleInfo, OsArgs,
        OsInner, OsInstanceArcBox, Process,
    },
    types::Address,
};
use serde::Deserialize;

thread_local! {
    static OS: RefCell<OsInstanceArcBox<'static>> = RefCell::new(os_from_file(config_path()));
    static MAPPED_RANGES: RefCell<Option<Vec<(Address, Address)>>> = RefCell::new(None);
    static CURRENT_PROCESS: RefCell<Option<IntoProcessInstanceArcBox<'static>>> = RefCell::new(None);
}

#[derive(Deserialize, Debug)]
struct MemflowConfig {
    /// Optional path to a directory with memflow plugins, if none then uses default scan locations.
    scan_path: Option<PathBuf>,
    /// Connector type (i.e. "kvm", "qemu", "native" etc...)
    conn: Option<String>,
    /// Arguments to pass to the connector.
    conn_args: Option<String>,
    /// OS type (i.e. "win32")
    os: String,
    /// Arguments to pass to the os.
    os_args: Option<String>,
}

// Adjusted from `yclass::config::config_path()`.
fn config_path() -> PathBuf {
    dirs::config_dir()
        .map(|dir| dir.join("yclass/memflow_config.toml"))
        .unwrap_or_else(|| "./memflow_config.toml".into())
}

fn os_from_file(path: PathBuf) -> OsInstanceArcBox<'static> {
    println!(
        "yclass-memflow: Attempting to create memflow OS from configuration file `{}`",
        path.display()
    );

    let mut buf = String::new();
    File::open(path)
        .expect("open config file")
        .read_to_string(&mut buf)
        .expect("read config to string");
    let config: MemflowConfig = toml::from_str(&buf).expect("config from toml buf");

    println!("yclass-memflow: {:#?}", config);

    let inventory = match config.scan_path {
        Some(path) => Inventory::scan_path(path).expect("inventory from `scan_path`"),
        None => Inventory::scan(),
    };
    let connector = config.conn.and_then(|name| {
        Some(
            inventory
                .create_connector(
                    &name,
                    None,
                    config
                        .conn_args
                        .and_then(|args| str::parse::<ConnectorArgs>(&args).ok())
                        .as_ref(),
                )
                .expect("connector created"),
        )
    });

    inventory
        .create_os(
            &config.os,
            connector,
            config
                .os_args
                .and_then(|args| str::parse::<OsArgs>(&args).ok())
                .as_ref(),
        )
        .expect("os created")
}

#[no_mangle]
pub extern "C" fn yc_attach(pid: u32) -> u32 {
    let proc = OS.with(|os| {
        os.borrow()
            .clone()
            .into_process_by_pid(pid)
            .expect("retrieve process from os")
    });

    CURRENT_PROCESS.with(move |curr_proc| {
        *curr_proc.borrow_mut() = Some(proc);
    });

    1
}

#[no_mangle]
pub unsafe extern "C" fn yc_read(address: usize, buffer: *mut u8, buffer_size: usize) -> u32 {
    // TODO: Why tf does it not care about AOB???
    CURRENT_PROCESS.with(|proc| {
        proc.borrow_mut().as_mut().unwrap().read_raw_into(
            address.into(),
            std::slice::from_raw_parts_mut(buffer, buffer_size),
        );
    });

    0
}

#[no_mangle]
pub extern "C" fn yc_can_read(address: usize) -> bool {
    MAPPED_RANGES.with(|mapped_ranges| {
        // Get valid memory pages then cache them.
        if mapped_ranges.borrow_mut().as_mut().is_none() {
            *mapped_ranges.borrow_mut() = Some(CURRENT_PROCESS.with(|curr_proc| {
                let mut ranges: Vec<(Address, Address)> = Vec::new();
                let callback = &mut |info: ModuleInfo| {
                    ranges.push((info.base, info.base + info.size));
                    true
                };
                curr_proc
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .module_list_callback(None, callback.into())
                    .unwrap();
                ranges
            }));
        }

        let address: Address = address.into();
        mapped_ranges
            .borrow()
            .as_ref()
            .unwrap()
            .iter()
            .any(|(start, end)| address >= *start && address <= *end)
    })
}

#[no_mangle]
pub extern "C" fn yc_detach() {
    CURRENT_PROCESS.with(|curr_proc| {
        *curr_proc.borrow_mut() = None;
    });
    MAPPED_RANGES.with(|mapped_ranges| {
        *mapped_ranges.borrow_mut() = None;
    });
}
