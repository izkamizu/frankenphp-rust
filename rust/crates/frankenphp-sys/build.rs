use std::{env, path::PathBuf};

fn php_include_flags(inc: &str) -> Vec<String> {
    vec![
        format!("-I{inc}"),
        format!("-I{inc}/main"),
        format!("-I{inc}/TSRM"),
        format!("-I{inc}/Zend"),
    ]
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest.ancestors().nth(2).unwrap(); // rust/ -> repo root
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // --- locate PHP headers + static lib -----------------------------------
    // Priority: $PHP_EMBED_DIR (with include/ and lib/libphp.a), else the
    // dev-provisioned /workspace/.php-embed + /tmp/php source tree.
    let (include_dir, lib_dir, lib_name) = match env::var("PHP_EMBED_DIR") {
        Ok(d) => {
            let d = PathBuf::from(d);
            (d.join("include"), d.join("lib"), "php".to_string())
        }
        Err(_) => {
            // fallback used by the Batch-1 dev environment
            let inc = "/tmp/php-8.3.14/include/php".to_string();
            let lib = workspace_root.join(".php-embed");
            (PathBuf::from(inc), lib, "php".to_string())
        }
    };

    assert!(
        include_dir.join("main/php.h").exists(),
        "PHP embed headers not found at {} — set PHP_EMBED_DIR",
        include_dir.display()
    );
    println!("cargo:rerun-if-env-changed=PHP_EMBED_DIR");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // --- compile the shim ----------------------------------------------------
    let mut cc = cc::Build::new();
    for f in php_include_flags(&include_dir.display().to_string()) {
        cc.flag(f);
    }
    // ZTS off (FrankenPHP uses non-ZTS, one request per thread model)
    // match the flags libphp.a itself was compiled with
    cc.define("ZEND_ENABLE_STATIC_TSRMLS_CACHE", "1");
    cc.define("_GNU_SOURCE", "1");
    cc.define("HAVE_MEMMOVE", "1");
    cc.file(manifest.join("shim/frankenphp_shim.c"));
    cc.compile("frankenphp_shim");

    // --- bindgen for the shim's public surface -------------------------------
    let bindings = bindgen::Builder::default()
        .header(
            manifest
                .join("shim/frankenphp_shim.h")
                .to_string_lossy()
                .into_owned(),
        )
        .clang_args(php_include_flags(&include_dir.display().to_string()))
        .allowlist_function("fp_.*")
        .generate()
        .expect("bindgen failed");
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("couldn't write bindings");

    // --- link libphp.a (static embed SAPI) ----------------------------------
    println!("cargo:rustc-link-lib=static={lib_name}");
    // system deps of a --disable-all embed build (see ./configure libs)
    println!("cargo:rustc-link-lib=dylib=resolv");
    println!("cargo:rustc-link-lib=dylib=crypt");
    println!("cargo:rustc-link-lib=dylib=sqlite3");
    println!("cargo:rustc-link-lib=dylib=ssl");
    println!("cargo:rustc-link-lib=dylib=crypto");
    println!("cargo:rustc-link-lib=dylib=z");
    println!("cargo:rustc-link-lib=dylib=onig");
    println!("cargo:rustc-link-lib=dylib=m");
    println!("cargo:rustc-link-lib=dylib=pthread");
    println!("cargo:rustc-link-lib=dylib=xml2");
}
