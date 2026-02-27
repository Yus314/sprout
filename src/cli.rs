use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sprout", version, about = "Evergreen note cultivation with spaced repetition")]
pub struct Cli {
    /// Path to notes vault (overrides config and current directory)
    #[arg(long, global = true)]
    pub vault: Option<PathBuf>,

    /// Output format
    #[arg(long, global = true, default_value = "human")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List notes due for review today
    Review,
    /// Mark a note as reviewed with a difficulty rating
    Done {
        /// Path to the reviewed note file
        file: PathBuf,
        /// Difficulty rating
        rating: Rating,
    },
    /// Change the maturity level of a note
    Promote {
        /// Path to the note file
        file: PathBuf,
        /// Target maturity level
        maturity: Maturity,
    },
    /// Show statistics about your note collection
    Stats,
    /// Add sprout frontmatter to a new or existing note
    Init {
        /// Path to the note file
        file: PathBuf,
    },
    /// List all tracked notes
    List {
        /// Filter by maturity level
        #[arg(long)]
        maturity: Option<Maturity>,
    },
    /// Show detailed information about a single note
    Show {
        /// Path to the note file
        file: PathBuf,
    },
    /// Open an existing note or create a new one
    Note {
        /// Title for a new note (omit to list all notes)
        title: Option<String>,
        /// Template name to use
        #[arg(long)]
        template: Option<String>,
    },
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum Rating {
    Hard,
    Good,
    Easy,
}

impl std::fmt::Display for Rating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rating::Hard => write!(f, "hard"),
            Rating::Good => write!(f, "good"),
            Rating::Easy => write!(f, "easy"),
        }
    }
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Maturity {
    Seedling,
    Budding,
    Evergreen,
}

impl std::fmt::Display for Maturity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Maturity::Seedling => write!(f, "seedling"),
            Maturity::Budding => write!(f, "budding"),
            Maturity::Evergreen => write!(f, "evergreen"),
        }
    }
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Human,
    Json,
}
