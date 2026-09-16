fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=core/hash.h");
    println!("cargo:rerun-if-changed=core/hash.c");
    println!("cargo:rerun-if-changed=core/serialize.h");
    println!("cargo:rerun-if-changed=core/serialize.c");
    println!("cargo:rerun-if-changed=core/messenger_core.h");
    println!("cargo:rerun-if-changed=core/messenger_core.c");
    println!("cargo:rerun-if-changed=core/mnemonic.h");
    println!("cargo:rerun-if-changed=core/mnemonic.c");

    cc::Build::new()
        .include("core")
        .file("core/hash.c")
        .file("core/serialize.c")
        .file("core/messenger_core.c")
        .file("core/mnemonic.c")
        .compile("kademlia_cryptography");
}
