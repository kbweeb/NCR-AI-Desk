# Install git hook that removes Cursor co-author from commit messages.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$src = Join-Path $root "scripts\prepare-commit-msg"
$dest = Join-Path $root ".git\hooks\prepare-commit-msg"
Copy-Item -Force $src $dest
Write-Host "Installed prepare-commit-msg hook -> $dest"
