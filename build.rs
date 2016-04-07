#[cfg(not(feature = "serde_macros"))]
mod inner {
    extern crate syntex;
    extern crate serde_codegen;

    use std::env;
    use std::path::Path;
    use std::fs;

    pub fn main() {
        let out_dir = env::var_os("OUT_DIR").unwrap();

        let src = Path::new("src/messages/types.rs.in");
        let message_path = Path::new(&out_dir).join("messages");
        let dst = Path::new(&out_dir).join("messages").join("types.rs");
        let mut registry = syntex::Registry::new();

        serde_codegen::register(&mut registry);
        let _ = fs::create_dir(message_path);
        registry.expand("", &src, &dst).unwrap();
    }
}

#[cfg(feature = "serde_macros")]
mod inner {
    pub fn main() {}
}

fn main() {
    inner::main();
}
