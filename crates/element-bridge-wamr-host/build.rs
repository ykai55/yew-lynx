use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if env::var_os("CARGO_FEATURE_WAMR").is_none() {
        return;
    }

    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let wamr = manifest.join("../../third_party/wasm-micro-runtime");
    let expected_revision = "25bd7eb63e828e4bd242cc9b38d260b4b31c6605";
    if !wamr.join("core/iwasm/include/wasm_export.h").is_file() {
        panic!("WAMR submodule is missing; expected revision {expected_revision}");
    }
    let actual_revision = Command::new("git")
        .args([
            "-C",
            wamr.to_str().expect("UTF-8 WAMR path"),
            "rev-parse",
            "HEAD",
        ])
        .output()
        .expect("git is required to verify the WAMR revision");
    if !actual_revision.status.success() {
        panic!("git could not verify the WAMR revision");
    }
    let actual_revision = String::from_utf8(actual_revision.stdout)
        .expect("WAMR revision must be UTF-8")
        .trim()
        .to_owned();
    if !actual_revision.eq(expected_revision) {
        panic!("WAMR revision mismatch: expected {expected_revision}, got {actual_revision}");
    }
    let checkout_status = Command::new("git")
        .args([
            "-C",
            wamr.to_str().expect("UTF-8 WAMR path"),
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
        ])
        .output()
        .expect("git is required to verify the WAMR checkout");
    if !checkout_status.status.success() {
        panic!("git could not verify whether the WAMR checkout is clean");
    }
    if !checkout_status.stdout.is_empty() {
        panic!("WAMR checkout must be clean at pinned revision {expected_revision}");
    }

    let shared = wamr.join("core/shared");
    let iwasm = wamr.join("core/iwasm");
    let wasi = iwasm.join("libraries/libc-wasi");
    let common = iwasm.join("common");
    let interpreter = iwasm.join("interpreter");
    let mut build = cc::Build::new();
    build
        .std("gnu99")
        .warnings(false)
        .flag_if_supported("-ffunction-sections")
        .flag_if_supported("-fdata-sections")
        .define("WASM_ENABLE_INTERP", "1")
        .define("WASM_ENABLE_FAST_INTERP", "0")
        .define("WASM_ENABLE_LIBC_BUILTIN", "1")
        .define("WASM_ENABLE_LIBC_WASI", "1")
        .define("WASM_ENABLE_MODULE_INST_CONTEXT", "1")
        .define("WASM_ENABLE_MULTI_MODULE", "0")
        .define("WASM_ENABLE_BULK_MEMORY", "1")
        .define("WASM_ENABLE_SHARED_MEMORY", "0")
        .define("WASM_ENABLE_MINI_LOADER", "0")
        .define("WASM_DISABLE_WAKEUP_BLOCKING_OP", "0")
        .define("WASM_ENABLE_SIMD", "0")
        .define("WASM_ENABLE_REF_TYPES", "1")
        .define("WASM_GLOBAL_HEAP_SIZE", "10485760")
        .define("BH_MALLOC", "wasm_runtime_malloc")
        .define("BH_FREE", "wasm_runtime_free")
        .include(wamr.join("core"))
        .include(iwasm.join("include"))
        .include(&common)
        .include(&interpreter)
        .include(shared.join("include"))
        .include(shared.join("mem-alloc"))
        .include(shared.join("utils"))
        .include(shared.join("platform/include"))
        .include(shared.join("platform/common/posix"))
        .include(shared.join("platform/common/libc-util"))
        .include(iwasm.join("libraries/libc-builtin"))
        .include(wasi.join("sandboxed-system-primitives/include"))
        .include(wasi.join("sandboxed-system-primitives/src"));

    let (disable_hw_bound_check, disable_stack_hw_bound_check) =
        match env::var("CARGO_CFG_TARGET_OS").expect("target OS").as_str() {
            "linux" => {
                build.define("BH_PLATFORM_LINUX", None);
                build.define("WASM_HAVE_MREMAP", "1");
                build.define("_GNU_SOURCE", None);
                build.include(shared.join("platform/linux"));
                add_c_files(&mut build, &shared.join("platform/linux"), true);
                add_c_files(&mut build, &shared.join("platform/common/posix"), true);
                add_c_files(&mut build, &shared.join("platform/common/libc-util"), true);
                ("0", "0")
            }
            "android" => {
                build.define("BH_PLATFORM_ANDROID", None);
                build.define("WASM_HAVE_MREMAP", "1");
                build.define("_GNU_SOURCE", None);
                build.include(shared.join("platform/android"));
                add_c_files(&mut build, &shared.join("platform/android"), true);
                add_c_files(&mut build, &shared.join("platform/common/posix"), true);
                add_c_files(&mut build, &shared.join("platform/common/libc-util"), true);
                // Android manages the main thread stack guard; WAMR must not retouch it.
                ("0", "1")
            }
            other => panic!("the WAMR host does not support target OS {other}"),
        };
    build.define("WASM_DISABLE_HW_BOUND_CHECK", disable_hw_bound_check);
    build.define(
        "WASM_DISABLE_STACK_HW_BOUND_CHECK",
        disable_stack_hw_bound_check,
    );
    let invoke_native = match env::var("CARGO_CFG_TARGET_ARCH")
        .expect("target architecture")
        .as_str()
    {
        "x86_64" => {
            build.define("BUILD_TARGET_X86_64", None);
            common.join("arch/invokeNative_em64.s")
        }
        "aarch64" => {
            build.define("BUILD_TARGET_AARCH64", None);
            build.define("BUILD_TARGET", Some("\"AARCH64\""));
            common.join("arch/invokeNative_aarch64.s")
        }
        other => panic!("the WAMR host does not support target architecture {other}"),
    };

    add_c_files(&mut build, &shared.join("mem-alloc"), true);
    add_c_files(&mut build, &shared.join("utils"), false);
    add_c_files(&mut build, &iwasm.join("libraries/libc-builtin"), false);
    add_c_files(&mut build, &wasi, true);
    add_c_files(&mut build, &common, false);
    build.file(invoke_native);
    build
        .file(interpreter.join("wasm_loader.c"))
        .file(interpreter.join("wasm_runtime.c"))
        .file(interpreter.join("wasm_interp_classic.c"));
    build.compile("iwasm");

    println!("cargo:rerun-if-changed={}", wamr.display());
}

fn add_c_files(build: &mut cc::Build, directory: &Path, recursive: bool) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
        .map(|entry| entry.expect("directory entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if recursive && path.is_dir() {
            add_c_files(build, &path, true);
        } else if path.extension().is_some_and(|extension| extension == "c") {
            build.file(path);
        }
    }
}
