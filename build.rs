#[cfg(not(feature = "serde_macros"))]
mod inner {
    extern crate syntex;
    extern crate serde_codegen;

    use std::env;
    use std::path::Path;
    use std::fs;

    pub fn main() {
        let out_dir = env::var_os("OUT_DIR").unwrap();

        let src = Path::new("src/messages/types/mod.rs.in");
        let message_path = Path::new(&out_dir).join("messages");
        let dst = Path::new(&out_dir).join("messages").join("types").join("mod.rs");
        fs::create_dir(&message_path).unwrap();
        fs::create_dir(message_path.join("types")).unwrap();
        serde_codegen::expand(&src, &dst).unwrap();
    }
}

#[cfg(feature = "serde_macros")]
mod inner {
    pub fn main() {}
}

fn main() {
    inner::main();
}
