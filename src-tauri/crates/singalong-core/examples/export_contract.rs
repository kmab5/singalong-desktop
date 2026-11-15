fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&schemars::schema_for!(singalong_core::model::Contract))
            .unwrap()
    );
}
