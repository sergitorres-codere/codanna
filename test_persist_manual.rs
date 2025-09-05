use codanna::project_resolver::persist::{ResolutionIndex, ResolutionPersistence, ResolutionRules};
use std::path::Path;
use std::collections::HashMap;

fn main() {
    let codanna_dir = Path::new(".codanna");
    let persistence = ResolutionPersistence::new(codanna_dir);
    
    let mut index = ResolutionIndex::new();
    let tsconfig_path = Path::new("examples/typescript/tsconfig.json");
    
    // Add some test data
    index.update_sha(tsconfig_path, &codanna::project_resolver::Sha256Hash::from_bytes(&[42; 32]));
    index.add_mapping("src/**/*.ts", tsconfig_path);
    index.set_rules(tsconfig_path, ResolutionRules {
        base_url: Some("./".to_string()),
        paths: HashMap::from([
            ("@components/*".to_string(), vec!["src/components/*".to_string()]),
        ]),
    });
    
    // Save it
    persistence.save("typescript", &index).expect("Failed to save");
    
    println!("Saved resolution index");
}
