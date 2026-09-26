use hasami::Args;

#[derive(Args)]
struct Opts {
    /// Name
    #[hasami(shrot)]
    name: String,
}

fn main() {}
