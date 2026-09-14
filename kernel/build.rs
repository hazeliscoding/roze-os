// hand the linker our script no matter where cargo is invoked from.
// the script pins the kernel at the limine higher-half load address.

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-arg=-T{dir}/linker.ld");
    println!("cargo:rerun-if-changed=linker.ld");
}
