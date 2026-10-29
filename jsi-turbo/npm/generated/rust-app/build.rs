fn main() {
    cxx_build::bridges(vec!["src/lib.rs"]).flag_if_supported("-std=c++17").compile("rust-app");
}
