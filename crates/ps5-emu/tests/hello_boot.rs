//! Boot the ps5rs hello-world eboot to completion and prove the full ABI
//! path: the guest string literal is reached through native execution →
//! `libkernel::puts` import → captured argument register → guest virtual
//! address → [`Process::read_string`] → Rust `String`.
//!
//! The eboot is supplied externally and pointed at with the `PS5_SDK_HELLO`
//! environment variable, so this doubles as the "real guest execution works"
//! gate. The test self-skips when the variable is unset and is serialized
//! against the other guest-run tests through `GUEST_LOCK`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ps5_emu::nid::catalog;
use ps5_emu::{Emulator, Process};

static GUEST_LOCK: Mutex<()> = Mutex::new(());

/// This binary runs as its own process, so it must not share the default
/// load base with the other test binaries (their identity-mapped
/// reservations would collide).
const LOAD_BASE: u64 = 0x860000000;

fn hello_eboot() -> Option<PathBuf> {
    std::env::var("PS5_SDK_HELLO").ok().map(PathBuf::from)
}

fn prx_dir_for(eboot: &Path) -> PathBuf {
    let dir = eboot.parent().unwrap_or(Path::new("."));
    let direct = dir.join("sce_module");
    if direct.is_dir() {
        return direct;
    }
    dir.join("prx")
}

#[test]
fn sdk_hello_puts_reads_the_guest_string() {
    let _guard = GUEST_LOCK.lock().unwrap();

    let Some(eboot_path) = hello_eboot() else {
        eprintln!("skipping: PS5_SDK_HELLO not set");
        return;
    };

    let prx_dir = prx_dir_for(&eboot_path);
    let eboot_bytes = std::fs::read(&eboot_path)
        .unwrap_or_else(|e| panic!("failed to read hello eboot {}: {e}", eboot_path.display()));

    let provider = |name: &str| -> Option<Vec<u8>> {
        let direct = prx_dir.join(name);
        if direct.is_file() {
            return std::fs::read(direct).ok();
        }
        let suffixed = prx_dir.join(format!("{name}.prx"));
        if suffixed.is_file() {
            return std::fs::read(suffixed).ok();
        }
        None
    };

    let process = Process::load_at("eboot.elf", eboot_bytes, provider, None, LOAD_BASE)
        .expect("failed to load hello eboot through loader pipeline");

    let mut emulator = Emulator::new(process);
    emulator
        .resolve_imports_with(&catalog())
        .expect("import table build failed");

    let report = emulator.run().expect("hello should run to completion");
    assert_eq!(report.exit_code, 0, "hello should exit with code 0");

    let puts = report
        .import_calls
        .iter()
        .find(|call| call.library == "libkernel" && call.name == "puts")
        .expect("expected a libkernel::puts import call");

    let text = emulator
        .process()
        .read_string(puts.args[0])
        .expect("puts string address should be readable in guest memory");
    assert_eq!(text, "Hello from ps5rs!");
}
