use clap::{Parser, Subcommand, ValueEnum};
use std::process;

mod aws;
mod commands;
mod crypto;
mod display;
mod error;
mod fs_utils;
mod input;
mod logging;
mod password;
mod safe;
mod shell;
mod types;
mod validation;

use error::SkitError;

#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Table,
    Json,
    Env,
    Postman,
    Terraform,
}

#[derive(Parser)]
#[command(name = "skit")]
#[command(
    about = "Security Kit (skit) - A secure secrets management tool for development environments"
)]
#[command(
    long_about = "skit stores secrets in .env format with encrypted values, making it safe to commit to git.\n\nUsage: skit [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]\nExample: skit --safe myproject init --generate"
)]
#[command(version = env!("SKIT_VERSION"))]
struct Cli {
    #[arg(
        short = 's',
        long,
        default_value = ".env.safe",
        help = "Path to the safe file (global option)"
    )]
    safe: String,

    #[arg(
        short = 'o',
        long = "format",
        value_enum,
        default_value = "table",
        help = "Output format: table, json, env, terraform, or postman (default: table) (global option)"
    )]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Create a new safe with strong password protection")]
    Init {
        #[arg(
            short = 'r',
            long,
            help = "Remember the safe key for automatic authentication"
        )]
        remember: bool,
        #[arg(short = 'd', long, help = "Description for the safe")]
        description: Option<String>,
        #[arg(
            long = "ssm-prefix",
            alias = "ssm",
            help = "Default AWS SSM parameter prefix to associate with this safe (e.g., /app/dev/)"
        )]
        ssm_prefix: Option<String>,
    },

    #[command(about = "Add or update a secret (encrypted by default)")]
    Set {
        #[arg(help = "Secret key name")]
        key: String,
        #[arg(help = "Secret value")]
        value: String,
        #[arg(short = 'p', long, help = "Store as plain text instead of encrypted")]
        plain: bool,
    },

    #[command(about = "Get and decrypt a secret value")]
    Get {
        #[arg(help = "Secret key name to retrieve")]
        key: String,
    },

    #[command(about = "Display all secrets in organized format")]
    Print {
        #[arg(
            short = 'p',
            long,
            help = "Show only plain text values (no password required)"
        )]
        plain: bool,
        #[arg(
            short = 'e',
            long,
            help = "Show only encrypted values (requires password)"
        )]
        enc: bool,
    },

    #[command(about = "List all secret keys with their types (encrypted/plain)")]
    Keys,

    #[command(about = "Remove a secret from the safe")]
    Rm {
        #[arg(help = "Secret key name to remove")]
        key: String,
    },

    #[command(about = "Execute command with secrets injected as environment variables")]
    Exec {
        #[arg(last = true, help = "Command and arguments to execute")]
        command: Vec<String>,
    },

    #[command(about = "Show safe metadata and integrity status")]
    Status,

    #[command(about = "Rotate encryption keys (re-encrypt all secrets)")]
    Rotate,

    #[command(about = "List all safe files in current directory")]
    Ls,

    #[command(about = "Output secrets for shell sourcing")]
    Env,

    #[command(about = "Output secrets in KEY=value format for piping to external commands")]
    Export,

    #[command(about = "Remember safe key for easy access")]
    RememberSafekey,

    #[command(about = "Clean up old saved keys")]
    CleanupKeys {
        #[arg(
            long = "older-than-days",
            help = "Remove keys not accessed for N days (required)"
        )]
        older_than_days: u64,
        #[arg(long, help = "Show what would be removed without actually removing")]
        dry_run: bool,
    },

    #[command(about = "Import secrets from existing cleartext file into safe")]
    Import {
        #[arg(short = 'f', long = "file", help = "Path to the input file to import")]
        file: String,
        #[arg(
            long = "plain-keys",
            help = "Comma-separated list of keys to store as plain text (default: all keys are encrypted)"
        )]
        plain_keys: Option<String>,
    },

    #[command(about = "Copy an existing safe to a new safe with new encryption")]
    Copy {
        #[arg(help = "Destination safe path")]
        dest: String,
        #[arg(
            short = 'r',
            long,
            help = "Remember the safe key for automatic authentication"
        )]
        remember: bool,
        #[arg(short = 'd', long, help = "Description for the new safe")]
        description: Option<String>,
    },

    #[command(about = "AWS SSM Parameter Store integration")]
    Ssm {
        #[command(subcommand)]
        action: SsmAction,
    },
}

#[derive(Subcommand)]
enum SsmAction {
    #[command(about = "Pull parameters from AWS SSM Parameter Store into safe")]
    Pull {
        #[arg(
            long,
            help = "SSM parameter path prefix (e.g., /myapp/dev/). If omitted, uses the safe's stored prefix"
        )]
        prefix: Option<String>,
        #[arg(long, help = "AWS region (default: from AWS config)")]
        region: Option<String>,
        #[arg(long, help = "Replace all existing secrets (default: merge)")]
        replace: bool,
        #[arg(long, help = "Don't overwrite existing keys")]
        no_overwrite: bool,
        #[arg(long, help = "Show what would be pulled without actually pulling")]
        dry_run: bool,
    },
}

fn normalize_safe_path(safe_name: &str) -> String {
    // If it's already in the correct format (.*.safe), use as-is
    if safe_name.starts_with('.') && safe_name.ends_with(".safe") {
        return safe_name.to_string();
    }

    // If it already ends with .safe but doesn't start with dot, add dot
    if safe_name.ends_with(".safe") && !safe_name.starts_with('.') {
        return format!(".{}", safe_name);
    }
    format!(".{}.safe", safe_name)
}

fn resolve_format(cli_format: &OutputFormat) -> OutputFormat {
    cli_format.clone()
}

#[tokio::main]
async fn main() {
    logging::init_logging();

    let cli = Cli::parse();
    let safe_path = normalize_safe_path(&cli.safe);
    let format = resolve_format(&cli.format);

    let result: Result<(), SkitError> = match cli.command {
        Commands::Init {
            remember,
            description,
            ssm_prefix,
        } => commands::init(
            &safe_path,
            remember,
            description.as_deref(),
            ssm_prefix.as_deref(),
        ),
        Commands::Set { key, value, plain } => commands::set(&safe_path, &key, &value, plain),
        Commands::Get { key } => commands::get(&safe_path, &key),
        Commands::Print { plain, enc } => commands::print(&safe_path, &format, plain, enc),
        Commands::Keys => commands::keys(&safe_path, &format),
        Commands::Rm { key } => commands::rm(&safe_path, &key),
        Commands::Exec { command } => commands::exec(&safe_path, &command),
        Commands::Status => commands::status(&safe_path, &format),
        Commands::Rotate => commands::rotate(&safe_path),
        Commands::Ls => commands::ls(&format),
        Commands::Env => commands::env(&safe_path),
        Commands::Export => commands::export(&safe_path),
        Commands::RememberSafekey => commands::remember_safekey(&safe_path),
        Commands::CleanupKeys {
            older_than_days,
            dry_run,
        } => commands::cleanup_keys(older_than_days, dry_run),
        Commands::Import { file, plain_keys } => {
            commands::import(&safe_path, &file, plain_keys.as_deref())
        }
        Commands::Copy {
            dest,
            remember,
            description,
        } => {
            let dest_path = normalize_safe_path(&dest);
            commands::copy(&safe_path, &dest_path, remember, description.as_deref())
        }
        Commands::Ssm { action } => match action {
            SsmAction::Pull {
                prefix,
                region,
                replace,
                no_overwrite,
                dry_run,
            } => commands::ssm_pull(
                &safe_path,
                prefix.as_deref(),
                region,
                replace,
                no_overwrite,
                dry_run,
            ),
        },
    };

    if let Err(e) = result {
        tracing::error!("{}", e);
        process::exit(1);
    }
}
