//! Handler for `fotobuch project` subcommands

use anyhow::Context;
use anyhow::Result;
use photobook_solver::commands;
use std::path::PathBuf;

pub enum ProjectSubcommand {
    New {
        name: String,
        width: f64,
        height: f64,
        bleed: f64,
        parent_dir: Option<PathBuf>,
        quiet: bool,
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
        } => {
            let parent = parent_dir
                .as_ref()
                .map(|p| p.as_path())
                .unwrap_or_else(|| std::path::Path::new("."));

            let config = commands::project::new::NewConfig {
                name: name.clone(),
                width_mm: width,
                height_mm: height,
                bleed_mm: bleed,
                quiet,
            };

            let result = commands::project_new(parent, &config)?;

            println!("✅ Project '{}' created successfully!", name);
            println!("📁 Location: {}", result.project_root.display());
            println!("🌿 Branch: {}", result.branch);
            println!("📄 YAML: {}", result.yaml_path.display());
            println!("📝 Template: {}", result.typ_path.display());

            Ok(())
        }
        ProjectSubcommand::List => {
            let project_root = std::env::current_dir()
                .context("Failed to determine current directory")?;

            let projects = commands::project::project_list(&project_root)?;

            if projects.is_empty() {
                println!("ℹ️  No projects found.");
            } else {
                for project in projects {
                    let marker = if project.is_current { "* " } else { "  " };
                    let current_label = if project.is_current { " (current)" } else { "" };
                    println!("{}{:<15} {}{}", marker, project.name, project.branch, current_label);
                }
            }

            Ok(())
        }
        ProjectSubcommand::Switch { name } => {
            let project_root = std::env::current_dir()
                .context("Failed to determine current directory")?;

            commands::project::project_switch(&project_root, &name)?;
            println!("✅ Switched to project '{}'", name);

            Ok(())
        }
    }
}
