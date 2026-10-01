fn main() {
    println!("cargo:rerun-if-env-changed=GUPI_UPDATE_PUBLIC_KEY");
    println!("cargo:rerun-if-changed=src/foundation/updater/macos.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("src/foundation/updater/macos.m")
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("gupi_sparkle_bridge");
        println!("cargo:rustc-link-lib=framework=Cocoa");
    }
}
