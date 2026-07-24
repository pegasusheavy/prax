// Fixture: #[derive(Model)] with an unknown field-level #[prax(...)] key
// Expected diagnostic: "unknown field-level `#[prax(...)]` key"

#[derive(prax_codegen::Model)]
struct User {
    #[prax(id)]
    id: i32,
    #[prax(unqiue)]
    email: String,
}

fn main() {}
