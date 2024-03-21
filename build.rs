use std::{
    error::Error,
    ffi::{OsStr, OsString},
    fs,
    process::Command,
};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=res");

    for file in fs::read_dir("res")? {
        let path = file?.path();
        let extension = path.extension().map(OsStr::as_encoded_bytes);

        match extension {
            Some(b"frag") | Some(b"vert") => {
                // Compile GLSL to SPIR-V.

                let mut spv = OsString::from(&path);
                spv.push(".spv");

                let output = Command::new("glslangValidator")
                    .arg("-V")
                    .arg("-o")
                    .arg(spv)
                    .arg(path)
                    .output()?;

                if !output.status.success() {
                    panic!("{}", String::from_utf8_lossy(&output.stdout));
                }
            }

            _ => {}
        }
    }

    Ok(())
}
