// Fixture: #[derive(Model)] with an unknown struct-level #[prax(...)] key
// Expected diagnostic: "unknown struct-level `#[prax(...)]` key"

#[derive(prax_codegen::Model)]
#[prax(tabel = "users")]
struct User {
    #[prax(id)]
    id: i32,
}

fn main() {}
