mod avail;
mod schema;
mod db;

use db::DatabaseClient;
use schema::{DatabaseError, Record};
use std::io::{self, Write};
use std::str::FromStr;

/// Helper function to get current timestamp for logging
fn log_with_timestamp(message: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    println!("[{}] {}", timestamp, message);
}

enum Command {
    Add(String, String),
    Get(String),
    List,
    Exit,
    Help,
}

impl FromStr for Command {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".to_string());
        }

        match parts[0].to_lowercase().as_str() {
            "add" => {
                if parts.len() < 3 {
                    return Err("Invalid add command format. Usage: add <key> <value>".to_string());
                }
                
                let key = parts[1].to_string();
                let value = parts[2..].join(" ");

                Ok(Command::Add(key, value))
            }
            "get" => {
                if parts.len() != 2 {
                    return Err("Invalid get command format. Usage: get <key>".to_string());
                }

                Ok(Command::Get(parts[1].to_string()))
            }
            "list" => Ok(Command::List),
            "exit" | "quit" => Ok(Command::Exit),
            "help" => Ok(Command::Help),
            _ => Err(format!("Unknown command: {}", parts[0])),
        }
    }
}

async fn handle_command(
    db: &mut DatabaseClient,
    command: Command
) -> Result<(), DatabaseError> {
    match command {
        Command::Add(key, value) => {
            log_with_timestamp(&format!("Adding record with key: {}", key));

            let record = Record::new(key, value);
            db.add_record(record).await?;

            log_with_timestamp(&format!("Record added successfully"));
        }
        Command::Get(key) => {
            log_with_timestamp(&format!("Getting record with key: '{}'", key));

            match db.get_record(&key).await? {
                Some(record) => {
                    println!("Key: {}", record.key);
                    println!("Value: {}", record.value);
                    println!("Created: {}", record.created_at.to_rfc3339());
                    if let Some(updated) = record.updated_at {
                        println!("Updated At: {}", updated);
                    }
                }
                None => log_with_timestamp(&format!("No record found with key: '{}'", key)),
            }
        }
        Command::List => {
            let records = db.list_records().await?;

            if records.is_empty() {
                println!("No records found");
            } else {
                for record in records {
                    println!("Key: {}", record.key);
                    println!("Value: {}", record.value);
                    println!("Created: {}", record.created_at.to_rfc3339());

                    if let Some(updated) = record.updated_at {
                        println!("Updated At: {}", updated);
                    }

                    println!("---");
                }
            }
        }
        Command::Exit => {
            log_with_timestamp("Exiting application");
            std::process::exit(0);
        }
        Command::Help => {
            println!("\nAvailable commands:");
            println!("  add <key> <value>  - Add a new record or update existing one");
            println!("  get <key>          - Retrieve a record by key");
            println!("  list               - List all records");
            println!("  exit               - Exit the application");
            println!("  help               - Show this help message");
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log_with_timestamp("Starting Avail database application");

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        log_with_timestamp("Error: Invalid number of arguments");
        println!("Usage: cargo run -- <app_name> [block_range]");
        println!("  app_name:       The human-readable app name");
        println!("  block_range:    (Optional) How many blocks to look back when scanning");
        return Ok(());
    }

    let app_name = args[1].clone();
    log_with_timestamp(&format!("Resolving app name: '{}'", app_name));

    // Try to fetch app_id by name
    let app_id = match avail::does_app_id_exist_on_avail(&app_name).await {
        Ok(Some(id)) => {
            log_with_timestamp(&format!("Found existing app ID: {}", id));
            id
        }
        Ok(None) => {
            log_with_timestamp("App not found. Creating new app ID...");
    
            avail::create_app_id_on_avail(&app_name).await.map_err(|e| {
                let msg = format!("Error creating app ID: {:?}", e);
                log_with_timestamp(&msg);
                Box::<dyn std::error::Error>::from(msg)
            })?;
    
            // Fetch again after creation
            avail::does_app_id_exist_on_avail(&app_name)
                .await
                .map_err(|e| {
                    let msg = format!("Error fetching app ID after creation: {:?}", e);
                    log_with_timestamp(&msg);
                    Box::<dyn std::error::Error>::from(msg)
                })?
                .ok_or_else(|| {
                    let msg = "App ID should exist after creation, but wasn't found.".to_string();
                    log_with_timestamp(&msg);
                    Box::<dyn std::error::Error>::from(msg)
                })?
        }
        Err(e) => {
            let msg = format!("Error checking app ID: {:?}", e);
            log_with_timestamp(&msg);
            return Err(Box::<dyn std::error::Error>::from(msg));
        }
    };
    

    let block_range = if args.len() == 3 {
        Some(args[2].parse::<u32>().map_err(|_| {
            let msg = "block_range must be a valid number".to_string();
            log_with_timestamp(&msg);
            std::io::Error::new(std::io::ErrorKind::InvalidInput, msg)
        })?)
    } else {
        None
    };

    let block_limit = block_range.unwrap_or(10);
    log_with_timestamp(&format!("Block search limit: {} blocks", block_limit));
    log_with_timestamp(&format!("Configuration - App name: '{}', App ID: {}", app_name, app_id));
    log_with_timestamp("Connecting to Avail node...");

    let mut db = DatabaseClient::new(app_id, block_range).await.map_err(|e| {
        let msg = format!("Error initializing database client: {:?}", e);
        log_with_timestamp(&msg);
        std::io::Error::new(std::io::ErrorKind::Other, msg)
    })?;

    log_with_timestamp("Successfully connected to Avail node");
    log_with_timestamp("Database client initialized");

    println!("\nAvailable commands:");
    println!("  add <key> <value>  - Add a new record or update existing one");
    println!("  get <key>          - Retrieve a record by key");
    println!("  list               - List all records");
    println!("  exit               - Exit the application");
    println!("  help               - Show this help message");
    println!("\nEnter commands below:");

    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print!("> ");
        io::stdout().flush()?;
        input.clear();
        // Fix: StdinLock does not have read_line, use stdin.read_line instead
        stdin.read_line(&mut input)?;
    
        match Command::from_str(&input) {
            Ok(cmd) => {
                if let Err(e) = handle_command(&mut db, cmd).await {
                    log_with_timestamp(&format!("Error: {}", e));
                }
            }
            Err(e) => {
                log_with_timestamp(&format!("Command error: {}", e));
            }
        }
    }
    
}
