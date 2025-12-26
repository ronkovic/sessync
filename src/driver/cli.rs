//! CLI Argument Parsing
//!
//! CLIの引数解析

use clap::Parser;

/// セッションログをBigQueryにアップロードするCLI
#[derive(Parser, Debug, Clone)]
#[command(name = "sessync")]
#[command(about = "Upload Claude Code session logs to BigQuery", long_about = None)]
pub struct Args {
    /// Dry run mode - don't actually upload
    #[arg(long)]
    pub dry_run: bool,

    /// Automatic mode (called from session-end hook)
    #[arg(long)]
    pub auto: bool,

    /// Manual mode (called by user command)
    #[arg(long)]
    pub manual: bool,

    /// Upload logs from all projects instead of current project only
    #[arg(long)]
    pub all_projects: bool,

    /// Config file path
    #[arg(short, long, default_value = "./.claude/sessync/config.json")]
    pub config: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_default_config() {
        let args = Args::parse_from(["sessync"]);
        assert_eq!(args.config, "./.claude/sessync/config.json");
        assert!(!args.dry_run);
        assert!(!args.all_projects);
    }

    #[test]
    fn test_args_dry_run() {
        let args = Args::parse_from(["sessync", "--dry-run"]);
        assert!(args.dry_run);
    }

    #[test]
    fn test_args_all_projects() {
        let args = Args::parse_from(["sessync", "--all-projects"]);
        assert!(args.all_projects);
    }

    #[test]
    fn test_args_custom_config() {
        let args = Args::parse_from(["sessync", "-c", "/custom/config.json"]);
        assert_eq!(args.config, "/custom/config.json");
    }

    #[test]
    fn test_args_combined() {
        let args = Args::parse_from(["sessync", "--dry-run", "--all-projects", "--auto"]);
        assert!(args.dry_run);
        assert!(args.all_projects);
        assert!(args.auto);
    }
}
