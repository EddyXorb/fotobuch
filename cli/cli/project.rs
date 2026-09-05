//! Handler for `fotobuch project` subcommands

use anyhow::Context;
use anyhow::Result;
use fotobuch::commands;
use std::path::PathBuf;
use tracing::info;

const WELCOME_MESSAGE: &str = r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                           Welcome to fotobuch!                               ║
╚══════════════════════════════════════════════════════════════════════════════╝

Your new photobook project has been created! Here's what you need to know:

📁 Project Structure:
   - <name>.yaml: Contains your project configuration and layout
   - <name>.typ:  Typst template for rendering your photobook
   - .fotobuch/:  Cache directory (not tracked in git)

📝 Workflow:
   1. fotobuch add <photos>    - Add photos to your project
   2. fotobuch build           - Generate preview PDF
   3. fotobuch place <photo>   - Manually adjust photo placement
   4. fotobuch build release   - Generate final PDF for printing

🔧 Configuration:
   You can edit <name>.yaml and <name>.typ to customize your photobook.
   All changes in between two fotobuch-command calls are tracked in git, so you can undo anything!

💡 Tips:
   - The project directory can be renamed, but don't rename .yaml or .typ files
   - Use 'git log' to see your project history
   - Each project lives on its own branch: fotobuch/<name>

Happy photobook making! 📷✨
"#;

pub enum ProjectSubcommand {
    New {
        name: String,
        width: f64,
        height: f64,
        bleed: f64,
        parent_dir: Option<PathBuf>,
        quiet: bool,
        with_cover: bool,
        cover_width: Option<f64>,
        cover_height: Option<f64>,
        spine_grow_per_10_pages_mm: Option<f64>,
        spine_mm: Option<f64>,
        margin_mm: f64,
    },
    List,
    Switch {
        name: String,
    },
}

pub fn handle(command: ProjectSubcommand) -> Result<()> {
    match command {
        ProjectSubcommand::New {
            name,
            width,
            height,
            bleed,
            parent_dir,
            quiet,
            with_cover,
            cover_width,
            cover_height,
            spine_grow_per_10_pages_mm,
            spine_mm,
            margin_mm,
        } => {
            let parent = parent_dir
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new("."));

            let config = commands::project::new::NewConfig {
                name: name.clone(),
                width_mm: width,
                height_mm: height,
                bleed_mm: bleed,
                with_cover,
                cover_width_mm: cover_width,
                cover_height_mm: cover_height,
                spine_grow_per_10_pages_mm,
                spine_mm,
                margin_mm,
                base_config: None,
            };

            let output = commands::project::new(parent, &config)?;

            if !quiet {
                println!("{WELCOME_MESSAGE}");
            }

            info!("✅ Project '{}' created successfully!", name);
            info!("📁 Location: {}", output.result.project_root.display());
            info!("🌿 Branch: {}", output.result.branch);
            info!("📄 YAML: {}", output.result.yaml_path.display());
            info!("📝 Template: {}", output.result.typ_path.display());

            Ok(())
        }
        ProjectSubcommand::List => {
            let project_root =
                std::env::current_dir().context("Failed to determine current directory")?;

            let projects = commands::project::list(&project_root)?.result;

            if projects.is_empty() {
                info!("ℹ️  No projects found.");
            } else {
                for project in projects {
                    let marker = if project.is_current { "* " } else { "  " };
                    let current_label = if project.is_current { " (current)" } else { "" };
                    info!(
                        "{}{:<15} {}{}",
                        marker, project.name, project.branch, current_label
                    );
                }
            }

            Ok(())
        }
        ProjectSubcommand::Switch { name } => {
            let project_root =
                std::env::current_dir().context("Failed to determine current directory")?;

            commands::project::switch(&project_root, &name)?;
            info!("✅ Switched to project '{}'", name);

            Ok(())
        }
    }
}
