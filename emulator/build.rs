fn main() {
    let mut builder = vergen_git2::Emitter::default();
    builder
        .add_instructions(
            &vergen_git2::BuildBuilder::default().build_timestamp(true).build().unwrap(),
        )
        .unwrap();
    builder
        .add_instructions(&vergen_git2::Git2Builder::default().sha(true).build().unwrap())
        .unwrap();
    builder.emit().unwrap();
}
