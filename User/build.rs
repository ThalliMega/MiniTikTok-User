fn main() -> std::io::Result<()> {
    let builder = tonic_build::configure();
    builder.compile(&["proto/user.proto"], &["proto"])
}
