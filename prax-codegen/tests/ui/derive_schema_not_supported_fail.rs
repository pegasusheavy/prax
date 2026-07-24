// Fixture: #[derive(Model)] with #[prax(schema = "...")] — parsed but inert
// Expected diagnostic: "`schema` is not yet supported by the derive macro"

#[derive(prax_codegen::Model)]
#[prax(schema = "public")]
struct User {
    #[prax(id)]
    id: i32,
}

fn main() {}
