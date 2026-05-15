fn main() {
    prost_build::compile_protos(&["../../proto/moyuguard.proto"], &["../../proto/"])
        .expect("Failed to compile protobuf");
    tauri_build::build()
}
