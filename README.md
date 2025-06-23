# Avail Database

A public database implementation on Avail that allows storing and retrieving records using an application key (app name).

## Features

- Store and retrieve records using a key-value model
- App name (application key) based organization
- Automatic metadata tracking
- Simple CLI interface
- Configurable block search limit (for initialization)
- Efficient record search and retrieval

## Prerequisites

- Rust and Cargo installed
- A valid Avail account seed phrase (set in `.env` as `AVAIL_SEED_PHRASE`)
- **No need to run your own Avail node!** This application uses Avail's public light client API endpoints by default.

## Setup

1. Clone the repository:
   ```
   git clone https://github.com/yourusername/avail-database.git
   cd avail-database
   ```

2. Set your Avail seed phrase in a `.env` file:
   ```
   echo 'AVAIL_SEED_PHRASE="your avail seed phrase here"' > .env
   ```

   > **Note:** The application connects to Avail's public light client API at `https://api.lightclient.turing.avail.so` and `wss://turing-rpc.avail.so/ws` by default. No local node setup is required.

## Usage

Run the application with an app name parameter and an optional block search limit:

```
cargo run -- <app_name> [block_range]
```

- `app_name`: The human-readable name for your application (used as the database namespace)
- `block_range`: (Optional) How many blocks to look back when scanning for existing database metadata (default: 10)

### Block Range Parameter

The block range parameter controls how many blocks back the database will search to discover existing data:

- When specified, the database will search up to this many blocks back from the current height to find existing data
- If not specified, it defaults to 10 blocks
- If it finds an existing database, it will use that database's starting block for all operations
- If no existing database is found, it creates a new one at the current height
- This parameter is only used during database initialization and not for subsequent operations

Benefits of using block range:
- Controls how far back to look for your existing database when restarting
- Reduces startup time by limiting the initial search range
- Once your database is found, operations are fast regardless of block range

### Available Commands

Once the application is running, you can use the following commands:

- `add <key> <value>` - Add a new record or update an existing one
- `get <key>` - Retrieve a record by key
- `list` - List all records
- `exit` or `quit` - Exit the application
- `help` - Show help message

## Example

```
$ cargo run -- my_app
[2025-06-23 14:32:06.780] Starting Avail database application
[2025-06-23 14:32:06.781] Resolving app name: 'my_app'
[2025-06-23 14:32:08.568] App not found. Creating new app ID...
Application Key Created
445
[2025-06-23 14:32:21.611] Block search limit: 10 blocks
[2025-06-23 14:32:21.611] Configuration - App name: 'my_app', App ID: 445
[2025-06-23 14:32:21.611] Connecting to Avail node...
[2025-06-23 14:32:22.255] Searching for existing database (blocks 1948284..1948294)
[2025-06-23 14:32:51.704] No existing database found, creating new one at height 1948294
[2025-06-23 14:33:00.640] Created new database starting at block: 1948296
[2025-06-23 14:33:00.640] Successfully connected to Avail node
[2025-06-23 14:33:00.640] Database client initialized

Available commands:
  add <key> <value>  - Add a new record or update existing one
  get <key>          - Retrieve a record by key
  list               - List all records
  exit               - Exit the application
  help               - Show this help message

Enter commands below:
> add naruto {"name": "Naruto Uzumaki", "email": "naruto@konoha.com"}
[2025-06-23 14:33:06.676] Adding record with key: naruto
[2025-06-23 14:33:40.833] Record added successfully
> add robin {"name": "Robin", "email": "robin@adventure.com"}
[2025-06-23 14:34:10.996] Adding record with key: robin
[2025-06-23 14:34:40.694] Record added successfully
> get naruto
[2025-06-23 14:41:26.715] Getting record with key: 'naruto'
[2025-06-23 14:41:27.251] Searching for record with key 'naruto' (database start: 1948296, current height: 1948322)
[2025-06-23 14:42:01.425] Found record with key 'naruto' at height 1948322
Key: naruto
Value: {"name": "Naruto Uzumaki", "email": "naruto@konoha.com"}
Created: 2025-06-23T11:33:06.676542+00:00
> list
[2025-06-23 14:51:32.274] Listing all records (database start: 1948296, current height: 1948352)
[2025-06-23 14:53:57.985] Found 2 records
Key: naruto
Value: {"name": "Naruto Uzumaki", "email": "naruto@konoha.com"}
Created: 2025-06-23T11:33:06.676542+00:00
---
Key: robin
Value: {"name": "Robin", "email": "robin@adventure.com"}
Created: 2025-06-23T11:34:10.996655+00:00
---
> exit
[2025-06-23 14:33:00.640] Exiting application
```

## How It Works

1. **Database Initialization**:
   - When you start the database with an app name, it searches for existing database metadata
   - The block_range parameter controls how many blocks back to search for existing database metadata
   - If existing metadata is found, the database uses that metadata for all operations
   - If no existing metadata is found, a new database is created at the current block height

2. **Record Storage**:
   - Records are stored as blobs in the Avail blockchain
   - Each record includes a key, value, creation timestamp, and unique ID
   - Records are serialized to JSON before being stored
   - Metadata is maintained to track the number of records and update timestamps

3. **Record Retrieval**:
   - When retrieving records, the database searches from the database's start height to the current height
   - For key-based lookups, it returns the most recent matching record found
   - For listing all records, it collects the most recent version of each record
   - Only blocks that could contain your data are searched, making operations efficient

## Troubleshooting

- **Seed phrase errors**: Make sure you have set the `AVAIL_SEED_PHRASE` environment variable in your `.env` file
- **Connection errors**: Verify that you have a working internet connection. The app connects to Avail's public light client API endpoints by default.
- **App name errors**: Ensure your app name is unique and valid
- **Performance issues**: If searching for records is slow, use a smaller block_range value for initialization