# SKIT - Security Kit

SKIT (Security Kit) is a command-line utility for securely managing development secrets. It stores encrypted secrets in a simple .env-style format that's safe to commit to git, providing a secure alternative to plaintext .env files.

> **⚠️ Platform Note:** SKIT is primarily designed for Linux and macOS. While it can be compiled for Windows, some features (such as secure file permissions and shell detection) may have limited functionality on Windows systems.

## Features

- 🔐 **Secure encryption** using AES-256-GCM with Argon2 password hashing
- 📁 **.env-style format** familiar to developers with safe commit capability
- 📥 **Import existing files** - convert `.env` files to encrypted safes instantly
- 🚀 **Direct execution** with `exec` command - inject secrets without exposing them
- 🔄 **Salt rotation** for forward secrecy and security maintenance
- 👁️ **Mixed storage** - store both encrypted secrets and plain text variables
- 🔑 **Safe key management** - remember passwords for seamless authentication
- 🎯 **Multiple output formats** - table, JSON, environment, and Terraform formats
- ⚡ **Auto-generated passwords** - create secure 20-character shell-safe passwords
- 🛡️ **Security-first design** - never store plaintext passwords or keys in project directories

## Installation

```bash
cargo install --path .
```

## Quick Start

### Option 1: Import Existing .env File (Recommended)
```bash
# Convert your existing .env file to encrypted safe
skit import -f .env

# Or import with mixed encryption (some keys plain, others encrypted)
skit import -f .env --plain NODE_ENV,PORT

# View the imported secrets
skit print

# Execute your app with injected secrets
skit exec -- npm start
```

### Option 2: Create New Safe from Scratch
```bash
# Initialize a new safe with auto-generated password (recommended)
skit init -g -r

# Add secrets manually
skit set API_KEY your-secret-key
skit set PORT 3000 -p

# View all secrets (default table format)
skit print

# View as JSON or environment format using shorthand
skit -o json print
skit -o env print

# Execute command with secrets injected
skit exec -- npm start
```

## Commands Reference

### Global Options (Before Command)
These options affect multiple commands and must be placed **before** the command:

- `-s, --safe <name>` - Specify safe file name (default: `.env.safe`)
- `-o, --format <format>` - Output format: `table`, `json`, `env`, or `terraform` (default: `table`)

### Environment Variables
Set these environment variables to customize default behavior:

- `SKIT_FORMAT` - Default output format (`json` or `env`). Overridden by `--format` flag.
- `SKIT_SAFEKEY` - Safe key for authentication (use with `-s` to specify which safe)

**Usage Pattern:** `skit [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]`

### Shorthand Flags Summary

All major options support shorthand flags for faster typing:

| Long Form | Short | Description |
|-----------|-------|-------------|
| `--safe` | `-s` | Specify safe file |
| `--format` | `-o` | Output format (table, json, env, terraform) |
| `--file` | `-f` | Input file path (import) |
| `--generate` | `-g` | Generate secure password (init) |
| `--remember` | `-r` | Remember safe key (init) |
| `--description` | `-d` | Set safe description (init) |
| `--plain` | `-p` | Plain text mode (set/print) |
| `--plain-keys` | | Comma-separated list of plain text keys (import) |
| `--enc` | `-e` | Encrypted only mode (print) |

**Examples with Environment Variables:**
```bash
# Set default format to JSON
export SKIT_FORMAT=json
skit print          # Uses JSON format
skit keys          # Uses JSON format
skit status        # Uses JSON format

# Override with command-line flag (long or short form)
skit --format env print    # Uses env format, ignores SKIT_FORMAT
skit -o env print          # Same using shorthand

# Set default format to env for shell integration
export SKIT_FORMAT=env
skit print > .env          # Direct shell format output
eval "$(skit env)"        # Already outputs shell format

# Clear environment variable to return to default
unset SKIT_FORMAT
skit print                 # Back to default table format
```

### Safe Management Commands

#### `init` - Initialize a new safe
Creates a new safe file with password protection.

**Usage:**
```bash
skit init [-g] [-r] [-d <description>] [--ssm-prefix <prefix>]
skit -s myproject init [-g] [-r] [-d <description>] [--ssm-prefix <prefix>]
```

**Options:**
- `-g, --generate` - Generate a secure random password automatically (20 characters, shell-safe)
- `-r, --remember` - Remember the safe key for automatic authentication (works with both manual and generated passwords)
- `-d, --description <description>` - Set description for the safe (skips interactive prompt)
- `--ssm-prefix <prefix>` (alias: `--ssm`) - Store a default AWS SSM parameter prefix (e.g., `/myapp/dev/`) with the safe metadata

**Examples:**
```bash
# Manual password entry with confirmation
skit init

# Auto-generate secure password (recommended)
skit init -g

# Auto-generate password and remember it for easy access (most convenient)
skit init -g -r

# Create named safe with description using shorthand flags
skit -s myproject init -g -r -d "My project secrets"

# Long form still works
skit --safe myproject init --generate --remember --description "My project secrets"

# Initialize and associate a default SSM prefix
skit init --ssm-prefix /myapp/dev/
```

**Password Generation Features:**
- Generates 20-character passwords
- Uses shell-safe characters: `a-z A-Z 0-9 ^ - _ . * + = : ,`
- Avoids problematic characters like quotes, `$`, backticks, etc.
- Guarantees at least one character from each category
- Displays password for secure storage

## 🚨 CRITICAL: Safe Key Storage Security

**⚠️ NEVER store safe keys in your project directory or any git-tracked location!**

### Built-in Safe Key Storage

**Default Storage Location: `~/.config/skit/keys/`**

SKIT automatically stores safe keys in `~/.config/skit/keys/` when you:
- Use the `remember-safekey` command
- Choose to save the safe key during `import` or `init` operations

Keys are stored as `~/.config/skit/keys/<uuid>.key` with restricted permissions (600), where `<uuid>` is the unique identifier for each safe. This default location ensures your keys are:
- **Stored outside your project directory** - never accidentally committed
- **Protected with secure file permissions** - only your user can read them (600)
- **Automatically loaded** - SKIT finds them when you run commands

### ✅ Alternative Methods to Store Generated Safe Keys:

**Method 1: System Keychain/Credential Manager**
```bash
# macOS Keychain
security add-generic-password -a $(whoami) -s "skit-myproject" -w "password"

# Linux Secret Service
secret-tool store --label="SKIT myproject" application skit project myproject

# Then retrieve when needed
SKIT_SAFEKEY=$(secret-tool lookup application skit project myproject) skit -s myproject print
```

**Method 2: Separate Private Directory**
```bash
# Create directory OUTSIDE any git repos
mkdir -p ~/.skit-passwords
echo 'your-generated-password' > ~/.skit-passwords/myproject.key
chmod 600 ~/.skit-passwords/myproject.key

# Use with:
SKIT_SAFEKEY=$(cat ~/.skit-passwords/myproject.key) skit -s myproject print
```

### 🚨 What NOT to Do:

❌ **NEVER do this:**
```bash
# DON'T: These will be committed to git!
echo 'password123' > .env.safe.password
echo 'password123' > password.txt
echo 'password123' > secrets/mypassword
```

❌ **Don't store in project directory:**
- Any file in your project can accidentally be committed
- Even `.gitignore` can be misconfigured or overridden


### Secret Management Commands

#### `set` - Add or update secrets
Stores a secret with encryption (default) or as plain text.

**Usage:**
```bash
skit set <KEY> <VALUE> [--plain]
```

**Arguments:**
- `<KEY>` - Secret key name
- `<VALUE>` - Secret value to store

**Options:**
- `-p, --plain` - Store as plain text instead of encrypted (no password required)

**Examples:**
```bash
# Store encrypted secret (prompts for password)
skit set API_KEY sk-1234567890abcdef
skit set DATABASE_URL postgres://user:pass@localhost/db

# Store plain text values (no password required)
skit set PORT 3000 -p
skit set BASE_URL https://api.example.com --plain

# Use with different safe using shorthand
skit -s myproject set SECRET_KEY myvalue
```

#### `import` - Import secrets from existing files
Converts cleartext files (like `.env`) into encrypted safe files, perfect for onboarding or migrating existing projects.

**Usage:**
```bash
skit import -f <FILE> [--plain-keys <KEYS>]
```

**Arguments:**
- `-f, --file <FILE>` - Path to the input file to import (required)

**Options:**
- `--plain-keys <KEYS>` - Comma-separated list of keys to store as plain text (default: all keys are encrypted)

**Behavior:**
- **Default:** All keys are encrypted if no flags specified
- **Auto-password generation:** Hit enter at password prompt to auto-generate secure password
- **Key saving:** Option to save safe key for passwordless future access
- **Safe naming:** Uses default `.env.safe` or specify with `--safe <name>`

**Examples:**
```bash
# Import all keys as encrypted (default behavior)
skit import -f .env

# Import to custom safe name
skit --safe myproject import -f .env

# Import with specific keys as plain text, others encrypted
skit import -f .env --plain-keys NODE_ENV,PORT,BASE_URL

# Import from different file types
skit import -f config.env --plain-keys NODE_ENV
```

**Sample Import Flow:**
```bash
$ skit import -f .env --plain-keys NODE_ENV,PORT

skit (Security Kit) - Finally safe to commit your secrets!
Let's convert your cleartext secrets to a secure safe.

📂 Found 4 secrets in .env
📋 2 keys will stay as plain text: NODE_ENV, PORT

🔑 Creating your secure safe...
Enter password for new safe (or hit enter to generate one automatically):
🎲 Generated Password: Kx3^mP.N.a25.X=rxfzt
Please save this password securely - you'll need it to access your safe!

✅ Import complete!
   4 secrets imported (2 encrypted, 2 plain text)
   Safe created: .env.safe

Save safe key for easy access? (y/N): y
✅ Safe key saved to ~/.config/skit/keys/uuid.key! No more password prompts needed.

🔐 Your secrets are now secure and safe to commit to git!
🚀 Try: skit print
```

**Migration Benefits:**
- **Zero setup time** - converts existing `.env` files instantly
- **Selective encryption** - choose which values need encryption
- **Git-safe immediately** - resulting `.env.safe` files are safe to commit
- **Team onboarding** - perfect for converting team projects to SKIT
- **Backwards compatible** - maintains `.env` format with encrypted values

#### `get` - Retrieve a secret
Displays the decrypted value of a specific secret.

**Usage:**
```bash
skit get <KEY>
```

**Arguments:**
- `<KEY>` - Secret key name to retrieve

**Examples:**
```bash
# Get encrypted secret (prompts for password)
skit get API_KEY

# Get from specific safe using shorthand
skit -s myproject get DATABASE_URL
```

### Viewing Commands

#### `print` - View all secrets
Displays all secrets in organized format with filtering options.

**Usage:**
```bash
skit print [--plain | --enc]
```

**Options:**
- `-p, --plain` - Show only plain text values (no password required)
- `-e, --enc` - Show only encrypted values (requires password)

**Global Options (use before `print`):**
- `--format <format>` - Output format: `table`, `json`, `env`, or `terraform` (default: `table`)

**Examples:**
```bash
# Show all secrets (default) - prompts for password if encrypted values exist
skit print

# Show only plain text values (no password required)
skit print -p

# Show only encrypted values (requires password)
skit print -e

# JSON format with filtering using shorthand
skit -o json print -p
skit -o json print -e

# Environment format for shell sourcing
skit -o env print > .env
skit -o env print -p > .env.plain

# Combined with different safe using shorthand
skit -s myproject print -e
```

**Behavior:**
- **Default:** Shows both encrypted and plain text values (prompts for password)
- **`-p, --plain`:** Shows only plain text values (no password prompt)
- **`-e, --enc`:** Shows only encrypted values (requires password)
- **Cannot combine:** `-p` and `-e` flags cannot be used together

#### `keys` - List secret names
Shows only the keys without values, organized by type (encrypted/plain).

**Usage:**
```bash
skit keys
```

**Global Options (use before `keys`):**
- `--format <format>` - Output format: `table` or `json` (default: `table`)

**Examples:**
```bash
# Default table format with type indicators
skit keys

# JSON format for scripts using shorthand
skit -o json keys

# List keys from specific safe using shorthand
skit -s myproject keys
```

### Execution Commands

#### `exec` - Execute with secrets
Runs a command with secrets injected as environment variables without exposing them in process lists.

**Usage:**
```bash
skit exec -- <COMMAND> [ARGS...]
```

**Arguments:**
- `--` - Separates SKIT options from the command to execute (required)
- `<COMMAND>` - Command to execute with injected secrets
- `[ARGS...]` - Arguments to pass to the command

**Examples:**
```bash
# Run Node.js application with secrets
skit exec -- npm start
skit exec -- node server.js

# Run Python application
skit exec -- python app.py

# Run with specific safe using shorthand
skit -s myproject exec -- docker-compose up

# Complex command with arguments
skit exec -- curl -H "Authorization: Bearer $API_KEY" https://api.example.com
```

**Security Features:**
- Secrets don't appear in `ps` output
- Environment variables are only available to the executed process
- Both encrypted and plain text variables are injected

### Maintenance Commands

#### `status` - Verify safe integrity
Checks that all secrets can be decrypted and displays safe statistics.

**Usage:**
```bash
skit status
```

**Global Options (use before `status`):**
- `--format <format>` - Output format: `table` or `json` (default: `table`)

**Examples:**
```bash
# Default format with integrity check
skit status

# JSON format for scripts and automation using shorthand
skit -o json status

# Check specific safe using shorthand
skit -s myproject status
```

**What it checks:**
- All encrypted secrets can be decrypted
- Password hash integrity
- Safe file format validity
- Statistics (total secrets, encrypted vs plain)

#### `rotate` - Rotate encryption
Rotates encryption keys and re-encrypts all secrets for forward secrecy.

**Usage:**
```bash
skit rotate
```

**Examples:**
```bash
# Rotate encryption for default safe
skit rotate

# Rotate specific safe using shorthand
skit -s myproject rotate
```

**What it does:**
- Prompts for current password
- Prompts for new password (or generates one)
- Re-encrypts all secrets with new salt
- Updates password hash
- Maintains all secret values

#### `rm` - Remove secret
Deletes a secret from the safe (prompts for password if removing encrypted secrets).

**Usage:**
```bash
skit rm <KEY>
```

**Arguments:**
- `<KEY>` - Secret key name to remove

**Examples:**
```bash
# Remove secret (prompts for password)
skit rm API_KEY

# Remove from specific safe using shorthand
skit -s myproject rm OLD_SECRET
```

#### `ls` - List available safes
Shows all `.safe` files in the current directory with statistics.

**Usage:**
```bash
skit ls
```

**Global Options (use before `ls`):**
- `--format <format>` - Output format: `table` or `json` (default: `table`)

**Examples:**
```bash
# Default format with statistics
skit ls

# JSON format for scripts using shorthand
skit -o json ls
```

**Output includes:**
- Safe file names
- Total number of secrets
- Number of encrypted vs plain text secrets
- Last modification time

#### `env` - Shell integration
Outputs secrets in shell-compatible format for direct sourcing into your shell.

**Usage:**
```bash
skit env
```

**Examples:**
```bash
# Source all secrets into current shell
eval "$(skit env)"

# Save to .env file for other tools
skit env > .env

# Use with specific safe using shorthand
eval "$(skit -s myproject env)"

# Pipe to other tools
skit env | grep DATABASE
```

**Output Format:**
Produces shell-compatible `KEY=VALUE` lines:
```bash
API_KEY=sk-1234567890abcdef
PORT=3000
NODE_ENV=development
```

### Safe Key Management Commands

#### `remember-safekey` - Remember safe key for easy access
Saves your safe key securely for automatic authentication, eliminating the need to enter passwords repeatedly.

**Usage:**
```bash
skit remember-safekey
```

**Examples:**
```bash
# Remember safe key for default safe
skit remember-safekey

# Remember safe key for specific safe using shorthand
skit -s myproject remember-safekey

# Remember safe key for production safe using shorthand
skit -s .production.safe remember-safekey
```

**What it does:**
- Prompts for your safe password to verify access
- Stores the password in `~/.config/skit/keys/<uuid>.key`
- Uses the safe's unique UUID for secure file naming
- Enables passwordless access for subsequent commands

**Security:**
- Keys are stored outside your project directory
- Each safe has a unique UUID-based filename
- Files are created with restricted permissions (600)
- Password verification ensures only valid keys are stored

## AWS SSM Parameter Store Integration

SKIT can pull parameters from AWS SSM Parameter Store, enabling teams to share secrets via AWS IAM while maintaining local encrypted storage for development.

### Use Cases

- **Team Collaboration**: Share secrets across team members using AWS IAM permissions instead of distributing safe keys
- **Production Secrets**: Pull production/staging parameters to local development environment
- **CI/CD Integration**: Access centralized secrets stored in SSM while keeping them encrypted locally
- **Multi-Environment**: Maintain separate SSM parameter paths for dev/staging/prod environments

### Prerequisites

1. **AWS Credentials**: Configure AWS credentials using one of these methods:
   - Environment variables: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`
   - AWS credentials file: `~/.aws/credentials`
   - IAM role (when running on EC2, ECS, Lambda, etc.)

2. **IAM Permissions**: Your AWS user/role needs these permissions:
   ```json
   {
     "Effect": "Allow",
     "Action": [
       "ssm:GetParametersByPath",
       "ssm:GetParameter"
     ],
     "Resource": "arn:aws:ssm:*:*:parameter/your-prefix/*"
   }
   ```

### Commands

#### `ssm pull` - Pull Parameters from SSM

Pull parameters from AWS SSM Parameter Store into your local safe.

**Usage:**
```bash
skit ssm pull [--prefix <PATH>] [OPTIONS]
```

**Options:**
- `--prefix <PATH>` - SSM parameter path prefix (e.g., `/myapp/dev/`). If omitted, SKIT uses the safe's stored `#@SSM_PREFIX`.
- `--region <REGION>` - AWS region (default: from AWS config)
- `--replace` - Replace all existing secrets (default: merge with existing)
- `--no-overwrite` - Don't overwrite existing keys (skip conflicts)
- `--dry-run` - Show what would be pulled without actually pulling

**Examples:**

```bash
# Pull all parameters from /myapp/dev/ prefix
skit ssm pull --prefix /myapp/dev/

# Pull from specific region
skit ssm pull --prefix /myapp/dev/ --region us-west-2

# Preview what would be pulled (dry run)
skit ssm pull --prefix /myapp/dev/ --dry-run

# Replace entire safe with SSM parameters
skit ssm pull --prefix /myapp/prod/ --replace

# Merge without overwriting existing keys
skit ssm pull --prefix /myapp/dev/ --no-overwrite

# Pull to a specific safe file
skit --safe .env.staging.safe ssm pull --prefix /myapp/staging/

# If a default prefix is stored (e.g., via `skit init --ssm-prefix /myapp/dev/`), you can simply run:
skit ssm pull
```

### Parameter Type Mapping

SKIT preserves SSM's security model by mapping parameter types:

| SSM Type | SKIT Storage | Description |
|----------|--------------|-------------|
| `SecureString` | Encrypted | KMS-encrypted in SSM → Re-encrypted with skit encryption locally |
| `String` | Plain text | Plain in SSM → Stored as plain text locally |
| `StringList` | Plain text | Comma-separated values treated as plain text |

**Example:**

If your SSM parameters are:
```
/myapp/dev/API_KEY          (SecureString)
/myapp/dev/DATABASE_URL     (SecureString)
/myapp/dev/NODE_ENV         (String)
/myapp/dev/PORT             (String)
```

After running `skit ssm pull --prefix /myapp/dev/`:
```bash
# Your .env.safe will contain:
#@SSM_PREFIX=/myapp/dev/
#@SSM_REGION=us-east-1

# SecureStrings are encrypted
API_KEY=ENC~v1~abc123...
DATABASE_URL=ENC~v1~def456...

# Strings are plain text
NODE_ENV=development
PORT=3000
```

### Safe Metadata

When you pull from SSM, SKIT stores metadata in the safe file:

```bash
#@SSM_PREFIX=/myapp/dev/
#@SSM_REGION=us-east-1
```

This metadata:
- Tracks where parameters came from
- Enables future bidirectional sync features
- Documents the SSM source for team members
- Can be set upfront during initialization via `skit init --ssm-prefix /myapp/dev/`

### Workflow Example

**Scenario:** Your team stores production secrets in AWS SSM, and you want to pull them for local development.

```bash
# 1. Configure AWS credentials (if not already configured)
aws configure

# 2. Pull parameters from SSM
skit ssm pull --prefix /myapp/dev/

# 3. Enter safe password to re-encrypt SecureString parameters
# (or use --remember during init to avoid password prompts)

# 4. View pulled secrets
skit print

# 5. Use secrets in your application
skit exec -- npm start
```

### Key Naming

SKIT strips the prefix from SSM parameter names by default:

| SSM Parameter Path | SKIT Key Name |
|-------------------|---------------|
| `/myapp/dev/API_KEY` | `API_KEY` |
| `/myapp/dev/database/host` | `database/host` |
| `/myapp/dev/database/port` | `database/port` |

**Note:** Nested paths (e.g., `database/host`) are preserved in the key name. This is valid in skit but results in keys with slashes.

### Security Considerations

1. **Re-encryption**: SecureString parameters are decrypted from SSM (using AWS KMS) and immediately re-encrypted using skit's AES-256-GCM encryption with your safe password.

2. **Local Storage**: After pulling, secrets are stored locally in encrypted form. This allows offline access and version control while maintaining security.

3. **AWS Credentials**: SKIT uses the AWS SDK's default credential provider chain. Ensure your AWS credentials are properly secured.

4. **IAM Permissions**: Grant least-privilege access. Only provide `ssm:GetParameter*` permissions for the specific parameter paths your team needs.

5. **Password Protection**: You'll need your safe password to re-encrypt SecureString parameters. Consider using `remember-safekey` for convenience.

### Troubleshooting

**Error: "No parameters found under prefix"**
- Verify the prefix exists in SSM: `aws ssm get-parameters-by-path --path /myapp/dev/`
- Ensure your prefix starts with `/`
- Check you're in the correct AWS region

**Error: "AWS authentication failed"**
- Verify AWS credentials: `aws sts get-caller-identity`
- Check IAM permissions for `ssm:GetParametersByPath`
- Ensure credentials match the region where parameters are stored

**Error: "Failed to decrypt SecureString"**
- Verify KMS key permissions in SSM Parameter Store
- Check your IAM role has `kms:Decrypt` permission for the KMS key

### Future Enhancements

Planned features for SSM integration:
- `ssm push` - Push local secrets to SSM Parameter Store
- `ssm sync` - Bidirectional sync with conflict resolution
- `ssm diff` - Compare local safe with SSM parameters
- Tag-based filtering for selective sync


## Output Formats

SKIT supports multiple output formats for better integration:

### Table Format (Default)
Grouped display separating encrypted and plain text secrets:
```
🔒 ENCRYPTED SECRETS (2)
├─ API_KEY: sk-1234567890abcdef
└─ DATABASE_URL: postgres://...

📝 PLAIN TEXT VALUES (2)
├─ PORT: 3000
└─ NODE_ENV: development
```

### JSON Format
Structured output for scripts and automation:
```json
{
  "items": [
    {
      "key": "API_KEY",
      "value": "sk-1234567890abcdef",
      "type": "ENC"
    },
    {
      "key": "PORT",
      "value": "3000",
      "type": "PLAIN"
    }
  ]
}
```

### Environment Format
Shell-compatible output for sourcing:
```bash
API_KEY=sk-1234567890abcdef
PORT=3000
NODE_ENV=development
```

### Terraform Format
HashiCorp Configuration Language (HCL) output for Terraform integration:
```hcl
variable "API_KEY" {
  type      = string
  sensitive = true
  default   = "sk-1234567890abcdef"
}

variable "PORT" {
  type    = string
  default = "3000"
}

variable "NODE_ENV" {
  type    = string
  default = "development"
}
```

## Safe File Format

Safes use a simple .env-style format with metadata stored in comments:

```bash
# ========================================
# SKIT SAFE METADATA - DO NOT EDIT
# ========================================
#@VERSION=1.0
#@DESCRIPTION=My project secrets
#@CREATED=2025-08-21 16:15:00 UTC
#@UPDATED=2025-08-21 16:25:00 UTC
#@PASS_HASH=$argon2id$v=19$m=19456,t=2,p=1$salt$hash
# ========================================
# SECRETS (KEY=VALUE or KEY=ENC~<data>)
# ========================================
PORT=3000
BASE_URL=https://api.example.com
API_KEY=ENC~salt123~dGVzdGVuY3J5cHRlZHZhbHVl
DATABASE_PASSWORD=ENC~salt456~YW5vdGhlcmVuY3J5cHRlZHZhbA==
```

- **Plain text** variables are stored as `KEY=value`
- **Encrypted** secrets are stored as `KEY=ENC~<salt>~<base64-encrypted-data>`
- **Metadata** uses `#@FIELD=value` format to avoid conflicts with user comments
- Files are safe to commit to version control

## Examples

The `examples/` directory contains demo applications:

### Node.js HTTP Server
A basic server that displays injected environment variables:

```bash
# Initialize safe with generated password (recommended for security)
skit init --generate

# Setup secrets
skit set API_KEY "sk-1234567890abcdef"
skit set API_URL "https://api.example.com" --plain

# Run server with injected secrets
skit exec -- node examples/server.js
```

Visit `http://localhost:3000` to see how SKIT securely injects secrets into your application.

## Security Features

- **Argon2** password hashing for verification
- **AES-256-GCM** encryption for secrets with per-secret salts
- **Salt rotation** capability for forward secrecy
- **Encryption-first design** - secrets never stored as plaintext
- **Safe process execution** - secrets don't appear in `ps` output
- **Git-safe format** - encrypted values safe to commit

## Building from Source

### Prerequisites

- Rust toolchain (install from https://rustup.rs)
- Git

### Build Instructions

```bash
# Clone the repository
git clone https://github.com/zstrikehq/skit.git
cd skit

# Build release version
cargo build --release

# Install locally
cargo install --path .

# Run tests
cargo test

# Run linter
cargo clippy
```

### Windows Builds

> **⚠️ Limited Windows Support:** SKIT is primarily designed for Unix-like systems (Linux/macOS). Windows builds are possible but come with the following limitations:
> - Secure file permissions (600) may not work as expected
> - Shell detection features have limited functionality
> - Some commands may behave differently due to platform differences
>
> We recommend using SKIT on Linux or macOS for the best experience, or via WSL (Windows Subsystem for Linux) on Windows.

For Windows, cross-compilation is supported from Linux:

```bash
# Install Windows target
rustup target add x86_64-pc-windows-gnu

# Install MinGW
sudo apt-get install gcc-mingw-w64-x86-64  # Ubuntu/Debian
# or
sudo dnf install mingw64-gcc                # Fedora

# Build Windows binary
cargo build --release --target x86_64-pc-windows-gnu
```

**Note on Windows Antivirus**: The release build profile is optimized to reduce false positives:
- Size optimization (`opt-level = "z"`)
- Link-time optimization (`lto = true`)
- Debug symbol stripping (`strip = true`)

For production Windows releases, consider code signing with a certificate from DigiCert, Sectigo, or GlobalSign to eliminate antivirus warnings.

## Contributing

Contributions are welcome! Please feel free to submit issues, fork the repository, and create pull requests.

### Development Setup

```bash
# Clone and setup
git clone https://github.com/zstrikehq/skit.git
cd skit

# Build and test
cargo build
cargo test
cargo clippy

# Format code
cargo fmt
```

### Guidelines

- Keep the code simple and focused
- Handle all error cases (avoid `.unwrap()`)
- Write tests for new features
- Follow Rust best practices
- Maintain backward compatibility with the .env format

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

Built with Rust using:
- `clap` for CLI parsing
- `aes-gcm` for encryption
- `argon2` for password hashing
- `crossterm` for secure password input
