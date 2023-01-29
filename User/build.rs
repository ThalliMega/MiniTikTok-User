fn main() -> std::io::Result<()> {
    let builder = tonic_build::configure();
    builder.compile(
        &["proto/user.proto", "proto/health_check.proto"],
        &["proto"],
    )?;

    let builder = tonic_build::configure().build_server(false);
    builder.compile(&["proto/auth.proto"], &["proto"])
}
