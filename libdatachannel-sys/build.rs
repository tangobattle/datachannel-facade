/// Mbed TLS libraries libdatachannel needs, in link (i.e. dependency) order.
const MBEDTLS_LIBS: [&str; 3] = ["mbedtls", "mbedx509", "mbedcrypto"];

fn rustc_link_search(cmake: &cmake::Config, path: &str) {
    let profile = cmake.get_profile();
    if cfg!(target_env = "msvc") {
        println!("cargo:rustc-link-search={path}/{profile}");
    } else {
        println!("cargo:rustc-link-search={path}")
    }
}

#[cfg(feature = "vendored")]
fn static_lib_name(lib: &str) -> String {
    if cfg!(target_env = "msvc") {
        format!("{lib}.lib")
    } else {
        format!("lib{lib}.a")
    }
}

/// Builds the vendored Mbed TLS submodule and returns its install prefix.
#[cfg(feature = "vendored")]
fn mbedtls_prefix(out_dir: &str) -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>> {
    // Same story as libdatachannel below: a submodule's sources are invisible to
    // Cargo's change detection, so point it at the directories we compile.
    println!("cargo:rerun-if-changed=mbedtls/include");
    println!("cargo:rerun-if-changed=mbedtls/library");

    // libdatachannel's DTLS transport uses the use_srtp extension unconditionally,
    // and Mbed TLS leaves that off in its stock configuration.
    let user_config = format!("{out_dir}/mbedtls_user_config.h").replace('\\', "/");
    std::fs::write(&user_config, "#define MBEDTLS_SSL_DTLS_SRTP\n")?;

    let prefix = cmake::Config::new("mbedtls")
        .out_dir(format!("{out_dir}/mbedtls"))
        .define("ENABLE_PROGRAMS", "OFF")
        .define("ENABLE_TESTING", "OFF")
        // Use the generated sources checked into the tree instead of re-running
        // the generator scripts, which would need Python.
        .define("GEN_FILES", "OFF")
        .define("MBEDTLS_FATAL_WARNINGS", "OFF")
        .define("MBEDTLS_USER_CONFIG_FILE", &user_config)
        .define("USE_SHARED_MBEDTLS_LIBRARY", "OFF")
        .define("USE_STATIC_MBEDTLS_LIBRARY", "ON")
        .build();

    // libdatachannel compiles against the installed headers without going
    // through Mbed TLS' CMake targets, so it would otherwise see the stock
    // configuration and disagree with the library we just built. Chain the same
    // user config onto the installed one. `install` restores the pristine header
    // whenever it runs, so this has to be redone after every build.
    let installed_config = prefix.join("include/mbedtls/mbedtls_config.h");
    let include_user_config = format!("\n#include \"{user_config}\"\n");
    let mut contents = std::fs::read_to_string(&installed_config)?;
    if !contents.ends_with(&include_user_config) {
        contents.push_str(&include_user_config);
        std::fs::write(&installed_config, contents)?;
    }

    Ok(Some(prefix))
}

/// Links the archives Mbed TLS was just built into. They are copied aside under
/// names nothing else answers to first: `-L` paths from `RUSTFLAGS` are searched
/// ahead of the ones a build script emits, so an Mbed TLS installed on the
/// system — a different major version, even — would otherwise shadow them.
#[cfg(feature = "vendored")]
fn link_mbedtls(
    prefix: Option<&std::path::Path>,
    out_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let prefix = prefix.expect("the vendored build always yields a prefix");
    let link_dir = std::path::Path::new(out_dir).join("mbedtls-link");
    std::fs::create_dir_all(&link_dir)?;

    for lib in MBEDTLS_LIBS {
        let vendored = format!("datachannel_{lib}");
        std::fs::copy(
            prefix.join("lib").join(static_lib_name(lib)),
            link_dir.join(static_lib_name(&vendored)),
        )?;
        println!("cargo:rustc-link-lib=static={vendored}");
    }

    println!("cargo:rustc-link-search=native={}", link_dir.display());
    Ok(())
}

/// Links a system Mbed TLS 3. Note that `-L` paths from `RUSTFLAGS` are searched
/// ahead of the one below, so on a machine carrying more than one Mbed TLS the
/// linker can still reach for a different install than the one MBEDTLS_DIR named
/// and CMake compiled against.
#[cfg(not(feature = "vendored"))]
fn link_mbedtls(
    prefix: Option<&std::path::Path>,
    _out_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(prefix) = prefix {
        println!("cargo:rustc-link-search=native={}/lib", prefix.display());
    }
    for lib in MBEDTLS_LIBS {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }
    Ok(())
}

/// Locates a system Mbed TLS 3. `MBEDTLS_DIR` points CMake and the linker at it
/// when it lives somewhere CMake does not search by default, e.g. Homebrew's
/// prefix; otherwise CMake's own search is left to it.
#[cfg(not(feature = "vendored"))]
fn mbedtls_prefix(_out_dir: &str) -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=MBEDTLS_DIR");
    Ok(std::env::var_os("MBEDTLS_DIR").map(std::path::PathBuf::from))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    // The vendored libdatachannel is a git submodule, so Cargo's default
    // "rerun if any package file changed" detection doesn't track its sources.
    // Watch the files we patch (and the bindgen header) explicitly so edits to
    // the C/C++ side actually trigger a rebuild + bindings regeneration.
    for path in [
        "libdatachannel/include/rtc/rtc.h",
        "libdatachannel/include/rtc/configuration.hpp",
        "libdatachannel/src/capi.cpp",
        "libdatachannel/src/peerconnection.cpp",
        "libdatachannel/src/impl/icetransport.cpp",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }

    let mut cmake = cmake::Config::new("libdatachannel");
    cmake.build_target("datachannel-static");
    cmake.out_dir(&out_dir);

    cmake.define("NO_WEBSOCKET", "ON");
    cmake.define("NO_EXAMPLES", "ON");
    cmake.define("NO_MEDIA", "ON");
    cmake.define("NO_TESTS", "ON");

    // Mbed TLS instead of OpenSSL: it covers everything libdatachannel needs
    // here (DTLS plus self-signed certificate generation) and builds in seconds
    // rather than minutes.
    cmake.define("USE_MBEDTLS", "ON");

    let mbedtls_prefix = mbedtls_prefix(&out_dir)?;
    if let Some(prefix) = &mbedtls_prefix {
        // MbedTLS_ROOT rather than CMAKE_PREFIX_PATH: it outranks every other
        // search path, including the pkg-config hints FindMbedTLS falls back on,
        // which on a machine with several Mbed TLS installs can point elsewhere.
        cmake.define("MbedTLS_ROOT", prefix);
    }

    cmake.build();

    cpp_build::Config::new()
        .include(format!("{}/lib", out_dir))
        .build("src/lib.rs");

    rustc_link_search(&cmake, &format!("native={out_dir}/build/deps/libjuice"));
    println!("cargo:rustc-link-lib=static=juice-static");

    rustc_link_search(&cmake, &format!("native={out_dir}/build/deps/usrsctp/usrsctplib"));
    println!("cargo:rustc-link-lib=static=usrsctp");

    rustc_link_search(&cmake, &format!("native={out_dir}/build"));
    println!("cargo:rustc-link-lib=static=datachannel-static");

    // Mbed TLS comes last: it is datachannel-static that references it.
    link_mbedtls(mbedtls_prefix.as_deref(), &out_dir)?;

    let bindings = bindgen::Builder::default()
        .header("libdatachannel/include/rtc/rtc.h")
        .generate()?;

    let out_path = std::path::PathBuf::from(out_dir);
    bindings.write_to_file(out_path.join("bindings.rs"))?;

    Ok(())
}
