use anyhow::Result;
use std::path::Path;
use dforge_search::index_directory;

pub fn cmd_search(path: &Path, query: &str) -> Result<()> {
    println!("Indexing {}...", path.display());
    let index = index_directory(path)?;
    println!("Indexed {} files, {} trigrams", index.file_count(), index.trigram_count());

    let results = index.search(query);

    if results.is_empty() {
        println!("No results for '{}'", query);
        return Ok(());
    }

    println!("\n{} results for '{}':\n", results.len(), query);
    let mut last_file = String::new();
    for r in &results {
        if r.file != last_file {
            println!("\x1b[1m{}\x1b[0m", r.file);
            last_file = r.file.clone();
        }
        // Highlight matched query in content
        let highlighted = r.content.replace(query, &format!("\x1b[43m{}\x1b[0m", query));
        println!("  \x1b[36m{:4}\x1b[0m │ {}", r.line, highlighted);
    }
    Ok(())
}
