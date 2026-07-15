use neunode_core::kind::{Kind, KindCategory};

fn main() {
    let names = Kind::ALL
        .iter()
        .map(|kind| serde_json::to_string(kind).expect("Kind must serialize"))
        .collect::<Vec<_>>();

    println!("// Generated from neunode-core. Run `npm run generate:protocol` to update.");
    println!("// Do not edit manually.\n");
    println!("export const Kind = {{");
    for name in names {
        let name = name.trim_matches('"');
        println!("\t{name}: \"{name}\",");
    }
    println!("}} as const;\n");
    println!("export type Kind = (typeof Kind)[keyof typeof Kind];\n");
    println!("export const KindCategory = {{");
    for category in KindCategory::ALL {
        let name = serde_json::to_string(&category).expect("KindCategory must serialize");
        let name = name.trim_matches('"');
        println!("\t{name}: \"{name}\",");
    }
    println!("}} as const;\n");
    println!("export type KindCategory = (typeof KindCategory)[keyof typeof KindCategory];");
}
