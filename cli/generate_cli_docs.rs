//! Generates the CLI flag reference for mdBook from the clap CLI definition.
//!
//! Run with: `cargo run --example generate-cli-docs`
//! Output:   `docs/book/src/cli/reference-generated.md`

mod cli;

use cli::Cli;

fn main() {
    let output_path = "docs/book/src/cli/reference-generated.md";

    let markdown = clap_markdown::help_markdown::<Cli>();

    let content = format!(
        "# Full Flag Reference\n\n\
        > This page is auto-generated from the CLI source. \
        Run `cargo run --example generate-cli-docs` to regenerate.\n\n\
        <!-- AUTO-GENERATED: do not edit by hand -->\n\n\
        {markdown}"
    );

    std::fs::write(output_path, content)
        .unwrap_or_else(|e| panic!("Failed to write {output_path}: {e}"));

    println!("Written: {output_path}");
}
