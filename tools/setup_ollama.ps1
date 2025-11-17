# PowerShell setup script for Ollama AI Assistant on Windows

Write-Host "=== Ollama AI Assistant Setup ===" -ForegroundColor Cyan
Write-Host ""

# Check if Ollama is installed
$ollamaPath = Get-Command ollama -ErrorAction SilentlyContinue

if (-not $ollamaPath) {
    Write-Host "❌ Ollama not found" -ForegroundColor Red
    Write-Host ""
    Write-Host "Install Ollama for Windows:" -ForegroundColor Yellow
    Write-Host "  1. Download from https://ollama.ai/download/windows"
    Write-Host "  2. Run the installer"
    Write-Host "  3. Restart this script"
    Write-Host ""
    exit 1
}

Write-Host "✓ Ollama is installed" -ForegroundColor Green

# Check if Ollama service is running
try {
    $response = Invoke-WebRequest -Uri "http://localhost:11434/api/tags" -UseBasicParsing -TimeoutSec 2 -ErrorAction Stop
    Write-Host "✓ Ollama service is running" -ForegroundColor Green
}
catch {
    Write-Host "⚠️  Ollama service not running" -ForegroundColor Yellow
    Write-Host "Starting Ollama service..."

    # Start Ollama in background
    Start-Process -FilePath "ollama" -ArgumentList "serve" -WindowStyle Hidden
    Start-Sleep -Seconds 3

    try {
        $response = Invoke-WebRequest -Uri "http://localhost:11434/api/tags" -UseBasicParsing -TimeoutSec 2
        Write-Host "✓ Ollama service started" -ForegroundColor Green
    }
    catch {
        Write-Host "❌ Failed to start Ollama service" -ForegroundColor Red
        Write-Host "Try running 'ollama serve' manually" -ForegroundColor Yellow
        exit 1
    }
}

# Pull recommended models
Write-Host ""
Write-Host "Pulling AI models..." -ForegroundColor Cyan
Write-Host "  This may take a few minutes depending on your connection"
Write-Host ""

# Gemma 2 2B (fast, lightweight)
Write-Host "1. Pulling gemma2:2b (recommended for fast iteration)..." -ForegroundColor Yellow
& ollama pull gemma2:2b

# Optional: Larger models for better accuracy
$response = Read-Host "Pull gemma2:9b for better accuracy? (slower) [y/N]"
if ($response -eq "y" -or $response -eq "Y") {
    Write-Host "2. Pulling gemma2:9b..." -ForegroundColor Yellow
    & ollama pull gemma2:9b
}

$response = Read-Host "Pull deepseek-coder for advanced code analysis? [y/N]"
if ($response -eq "y" -or $response -eq "Y") {
    Write-Host "3. Pulling deepseek-coder:6.7b..." -ForegroundColor Yellow
    & ollama pull deepseek-coder:6.7b
}

# Install Python dependencies
Write-Host ""
Write-Host "Installing Python dependencies..." -ForegroundColor Cyan

# Check if Python is available
$pythonPath = Get-Command python -ErrorAction SilentlyContinue
if (-not $pythonPath) {
    Write-Host "❌ Python 3 not found" -ForegroundColor Red
    Write-Host "Please install Python 3.8 or later from https://python.org" -ForegroundColor Yellow
    exit 1
}

Write-Host "✓ Python 3 is installed" -ForegroundColor Green

# Install requests library
python -m pip install --user requests toml

Write-Host ""
Write-Host "✅ Setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Available models:" -ForegroundColor Cyan
& ollama list
Write-Host ""
Write-Host "Usage examples:" -ForegroundColor Cyan
Write-Host "  # Scan project for issues" -ForegroundColor White
Write-Host "  python tools/ollama_assistant.py --scan" -ForegroundColor Yellow
Write-Host ""
Write-Host "  # Auto-fix detected issues" -ForegroundColor White
Write-Host "  python tools/ollama_assistant.py --fix" -ForegroundColor Yellow
Write-Host ""
Write-Host "  # Watch mode (continuous monitoring)" -ForegroundColor White
Write-Host "  python tools/ollama_assistant.py --watch" -ForegroundColor Yellow
Write-Host ""
Write-Host "  # Analyze git diff" -ForegroundColor White
Write-Host "  python tools/ollama_assistant.py --diff" -ForegroundColor Yellow
Write-Host ""
Write-Host "For more options: python tools/ollama_assistant.py --help" -ForegroundColor Gray
