# Download Embedding Models for Codanna
#
# This script downloads embedding models from HuggingFace for use with Codanna.
# Useful when corporate network filtering blocks direct downloads during indexing.
#
# Usage:
#   .\download-model.ps1 -Model MultilingualE5Small
#   .\download-model.ps1 -Model AllMiniLML6V2 -CacheDir "C:\custom\path"
#
# Available Models:
#   - AllMiniLML6V2 (default, English-only, 384 dimensions)
#   - MultilingualE5Small (94 languages, 384 dimensions)
#   - MultilingualE5Base (94 languages, 768 dimensions)
#   - MultilingualE5Large (94 languages, 1024 dimensions)
#   - BGESmallZHV15 (Chinese-specialized, 512 dimensions)

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet(
        "AllMiniLML6V2",
        "MultilingualE5Small",
        "MultilingualE5Base",
        "MultilingualE5Large",
        "ParaphraseMLMiniLML12V2",
        "BGESmallZHV15",
        "BGEBaseENV15",
        "BGELargeENV15"
    )]
    [string]$Model = "AllMiniLML6V2",

    [Parameter(Mandatory=$false)]
    [string]$CacheDir = "$env:USERPROFILE\.codanna\models"
)

# Model repository mapping
# These must match exactly what fastembed expects
$modelRepos = @{
    "AllMiniLML6V2" = "Qdrant/all-MiniLM-L6-v2-onnx"
    "MultilingualE5Small" = "intfloat/multilingual-e5-small"
    "MultilingualE5Base" = "intfloat/multilingual-e5-base"
    "MultilingualE5Large" = "intfloat/multilingual-e5-large"
    "ParaphraseMLMiniLML12V2" = "Xenova/paraphrase-multilingual-MiniLM-L12-v2"
    "BGESmallZHV15" = "Xenova/bge-small-zh-v1.5"
    "BGEBaseENV15" = "Xenova/bge-base-en-v1.5"
    "BGELargeENV15" = "Xenova/bge-large-en-v1.5"
}

$repo = $modelRepos[$Model]
if (-not $repo) {
    Write-Error "Unknown model: $Model"
    exit 1
}

Write-Host "Downloading model: $Model" -ForegroundColor Cyan
Write-Host "Repository: $repo" -ForegroundColor Gray
Write-Host "Cache directory: $CacheDir" -ForegroundColor Gray
Write-Host ""

# Create cache directory if it doesn't exist
if (-not (Test-Path $CacheDir)) {
    New-Item -ItemType Directory -Path $CacheDir -Force | Out-Null
    Write-Host "Created cache directory: $CacheDir" -ForegroundColor Green
}

# Convert repo path for directory structure
$repoDir = $repo -replace "/", "--"
$modelPath = Join-Path $CacheDir "models--$repoDir"

# Check if model already exists
if (Test-Path $modelPath) {
    $response = Read-Host "Model directory already exists. Overwrite? (y/N)"
    if ($response -ne "y") {
        Write-Host "Download cancelled." -ForegroundColor Yellow
        exit 0
    }
}

# Base URL for HuggingFace
$baseUrl = "https://huggingface.co/$repo/resolve/main"

# Required files for ONNX models
# intfloat models expect onnx/ subdirectory, Qdrant models expect root level
$files = @(
    "config.json",
    "tokenizer.json",
    "tokenizer_config.json",
    "special_tokens_map.json",
    "onnx/model.onnx"
)

# Optional files (model.onnx_data is needed for Large models with split weights)
$optionalFiles = @(
    "onnx/model.onnx_data",
    "onnx/model_quantized.onnx"
)

Write-Host "Downloading files..." -ForegroundColor Cyan

# Fetch the commit hash first
Write-Host "Fetching latest commit hash..." -ForegroundColor Gray
try {
    $apiUrl = "https://huggingface.co/api/models/$repo/revision/main"
    $response = Invoke-RestMethod -Uri $apiUrl -ErrorAction Stop
    $commitHash = $response.sha
    Write-Host "  Commit hash: $commitHash" -ForegroundColor Green
} catch {
    Write-Host "  Warning: Could not fetch commit hash, using timestamp as fallback" -ForegroundColor Yellow
    $commitHash = "main"
}

# Create snapshots directory with commit hash
$snapshotDir = Join-Path $modelPath "snapshots\$commitHash"
New-Item -ItemType Directory -Path $snapshotDir -Force | Out-Null
Write-Host ""

# Download required files
foreach ($file in $files) {
    $url = "$baseUrl/$file"
    $destPath = Join-Path $snapshotDir $file
    $destDir = Split-Path $destPath -Parent

    # Create subdirectories if needed
    if (-not (Test-Path $destDir)) {
        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
    }

    Write-Host "  Downloading: $file" -ForegroundColor Gray

    try {
        Invoke-WebRequest -Uri $url -OutFile $destPath -ErrorAction Stop
        Write-Host "    ✓ Success" -ForegroundColor Green
    }
    catch {
        $errorMsg = $_.Exception.Message
        Write-Host "    ✗ Failed: $errorMsg" -ForegroundColor Red

        # Check for common corporate network issues
        if ($errorMsg -match "username.*password" -or $errorMsg -match "401" -or $errorMsg -match "403" -or $errorMsg -match "proxy" -or $errorMsg -match "authentication") {
            Write-Host "    ⚠ This looks like a corporate network/proxy issue" -ForegroundColor Yellow
            Write-Host "    Try running this script from a non-restricted network (home, mobile hotspot)" -ForegroundColor Yellow
        }

        Write-Host "    Manual download: $url" -ForegroundColor Gray

        # For critical files, exit with helpful message
        if ($file -eq "onnx/model.onnx") {
            Write-Host ""
            Write-Host "CORPORATE NETWORK DETECTED" -ForegroundColor Red
            Write-Host "HuggingFace downloads are blocked by your network." -ForegroundColor Yellow
            Write-Host ""
            Write-Host "Options:" -ForegroundColor Cyan
            Write-Host "  1. Run this script from home/mobile network, then copy files to work machine" -ForegroundColor Gray
            Write-Host "  2. Use the default AllMiniLML6V2 model (already cached)" -ForegroundColor Gray
            Write-Host "  3. Ask IT to whitelist huggingface.co" -ForegroundColor Gray
            Write-Host ""
            Write-Host "See docs/troubleshooting/corporate-networks.md for details" -ForegroundColor Cyan
            exit 1
        }
    }
}

# Download optional files (don't fail if these don't exist)
foreach ($file in $optionalFiles) {
    $url = "$baseUrl/$file"
    $destPath = Join-Path $snapshotDir $file
    $destDir = Split-Path $destPath -Parent

    if (-not (Test-Path $destDir)) {
        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
    }

    Write-Host "  Downloading (optional): $file" -ForegroundColor Gray

    try {
        Invoke-WebRequest -Uri $url -OutFile $destPath -ErrorAction Stop
        Write-Host "    ✓ Success" -ForegroundColor Green
    }
    catch {
        $errorMsg = $_.Exception.Message
        if ($errorMsg -match "username.*password" -or $errorMsg -match "401" -or $errorMsg -match "403" -or $errorMsg -match "proxy" -or $errorMsg -match "authentication") {
            Write-Host "    - Skipped (blocked by network)" -ForegroundColor DarkGray
        } else {
            Write-Host "    - Skipped (not available)" -ForegroundColor DarkGray
        }
    }
}

# Create refs directory and main file
$refsDir = Join-Path $modelPath "refs"
New-Item -ItemType Directory -Path $refsDir -Force | Out-Null
$commitHash | Out-File -FilePath (Join-Path $refsDir "main") -Encoding ascii -NoNewline

Write-Host ""
Write-Host "Model downloaded successfully!" -ForegroundColor Green
Write-Host "Location: $modelPath" -ForegroundColor Gray
Write-Host ""
Write-Host "You can now use this model in your .codanna/settings.toml:" -ForegroundColor Cyan
Write-Host "[semantic_search]" -ForegroundColor Gray
Write-Host "model = `"$Model`"" -ForegroundColor Gray
