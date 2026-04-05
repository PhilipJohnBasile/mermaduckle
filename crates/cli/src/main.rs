use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use mermaduckle_sdk::Client;

#[derive(Parser, Debug)]
#[command(
    name = "mermaduckle",
    version = "0.1.0",
    about = "CLI for Mermaduckle AI Agent Orchestration"
)]
struct Cli {
    #[arg(long, default_value = "http://localhost:3000")]
    url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all workflows
    Workflows,
    /// Run a specific workflow
    Run {
        /// The ID of the workflow to run
        workflow_id: String,
    },
    /// List all agents
    Agents,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let client = Client::new(&cli.url);

    match &cli.command {
        Commands::Workflows => {
            println!("{} Fetching workflows...", "i".blue());
            let workflows = client.list_workflows().await?;
            if workflows.is_empty() {
                println!("No workflows found.");
            } else {
                for wf in workflows {
                    println!(
                        "{} ({}) - [{}]",
                        wf.name.green().bold(),
                        wf.id.dimmed(),
                        wf.status.blue()
                    );
                }
            }
        }
        Commands::Run { workflow_id } => {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message(format!("Running workflow {}...", workflow_id));
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            match client.run_workflow(workflow_id).await {
                Ok(res) => {
                    pb.finish_and_clear();
                    println!("{} Workflow run completed successfully!", "✔".green());
                    println!("Run ID: {}", res.run_id);
                    println!("Output:\n{}", res.output);
                }
                Err(e) => {
                    pb.finish_and_clear();
                    println!("{} Workflow run failed: {}", "✘".red(), e);
                }
            }
        }
        Commands::Agents => {
            println!("{} Fetching agents...", "i".blue());
            let agents = client.list_agents().await?;
            if agents.is_empty() {
                println!("No agents found.");
            } else {
                for ag in agents {
                    println!(
                        "{} ({}) - {} runs, {}% success",
                        ag.name.magenta().bold(),
                        ag.id.dimmed(),
                        ag.runs,
                        ag.success_rate
                    );
                }
            }
        }
    }

    Ok(())
}
