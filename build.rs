fn main() {
    //println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-cfg=-static-libgcc -static-libstdc++ -Wl,-Bstatic -lstdc++ -lpthread -Wl,-Bdynamic");
}
