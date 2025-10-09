#!/usr/bin/env bash
# Script to create /opt/local/bin and optionally add it to PATH

set -euo pipefail

TARGET_DIR="/opt/local/bin"
USER_NAME="${USER:-$(whoami)}"
SCRIPT_NAME="$(basename "$0")"

# Detect shell and corresponding initialization file
detect_shell_init_file() {
  local shell_name
  shell_name=$(basename "${SHELL:-}")

  case "$shell_name" in
    bash)
      # Prefer .bashrc, fallback to .bash_profile
      if [[ -f "$HOME/.bashrc" ]]; then
        echo "$HOME/.bashrc"
      else
        echo "$HOME/.bash_profile"
      fi
      ;;
    zsh)
      echo "$HOME/.zshrc"
      ;;
    *)
      # Default fallback
      echo "$HOME/.profile"
      ;;
  esac
}

# Check if /opt/local/bin is in PATH
is_in_path() {
  local dir="$1"
  IFS=':' read -ra path_array <<< "$PATH"
  for p in "${path_array[@]}"; do
    if [[ "$p" == "$dir" ]]; then
      return 0
    fi
  done
  return 1
}

main() {
  if [[ ! -d "$TARGET_DIR" ]]; then
    echo "Creating directory $TARGET_DIR with sudo..."
    sudo mkdir -p "$TARGET_DIR"
  else
    echo "Directory $TARGET_DIR already exists."
  fi

  # Check if directory is already owned by current user
  current_owner=$(stat -f "%Su" "$TARGET_DIR" 2>/dev/null || echo "")
  if [[ "$current_owner" != "$USER_NAME" ]]; then
    echo "Changing ownership of $TARGET_DIR to user $USER_NAME..."
    sudo chown "$USER_NAME" "$TARGET_DIR"
  else
    echo "Directory $TARGET_DIR is already owned by $USER_NAME."
  fi

  if is_in_path "$TARGET_DIR"; then
    echo "$TARGET_DIR is already in your PATH."
  else
    echo "$TARGET_DIR is not in your PATH."
    read -r -p "Would you like to add $TARGET_DIR to your PATH in your shell initialization file? (y/n) " answer
    case "$answer" in
      [Yy]* )
        local init_file
        init_file=$(detect_shell_init_file)
        echo "Adding $TARGET_DIR to PATH in $init_file..."

        # Backup the init file before modifying
        cp "$init_file" "${init_file}.bak.$(date +%Y%m%d%H%M%S)"

        # Append export line if not already present
        if ! grep -qF "$TARGET_DIR" "$init_file"; then
          echo "" >> "$init_file"
          echo "# Added by $SCRIPT_NAME to include $TARGET_DIR in PATH" >> "$init_file"
          echo "export PATH=\"\$PATH:$TARGET_DIR\"" >> "$init_file"
          echo "Added $TARGET_DIR to PATH in $init_file."
          echo "Please restart your terminal or run 'source $init_file' to apply changes."
        else
          echo "It seems $TARGET_DIR is already referenced in $init_file."
        fi
        ;;
      * )
        echo "Skipping PATH update."
        ;;
    esac
  fi
}

main
