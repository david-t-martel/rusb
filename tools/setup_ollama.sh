#!/bin/bash
# Setup script for Ollama AI Assistant

set -e

echo "=== Ollama AI Assistant Setup ==="
echo ""

# Check if Ollama is installed
if ! command -v ollama &> /dev/null; then
    echo "❌ Ollama not found"
    echo ""
    echo "Install Ollama:"
    echo "  macOS/Linux: curl -fsSL https://ollama.ai/install.sh | sh"
    echo "  Windows: Download from https://ollama.ai/download"
    echo ""
    exit 1
fi

echo "✓ Ollama is installed"

# Check if Ollama service is running
if ! curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "⚠️  Ollama service not running"
    echo "Starting Ollama service..."

    if command -v systemctl &> /dev/null; then
        sudo systemctl start ollama
    else
        ollama serve &
        sleep 2
    fi
fi

echo "✓ Ollama service is running"

# Pull recommended models
echo ""
echo "Pulling AI models..."
echo "  This may take a few minutes depending on your connection"
echo ""

# Gemma 2 2B (fast, lightweight)
echo "1. Pulling gemma2:2b (recommended for fast iteration)..."
ollama pull gemma2:2b

# Optional: Larger models for better accuracy
read -p "Pull gemma2:9b for better accuracy? (slower) [y/N]: " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "2. Pulling gemma2:9b..."
    ollama pull gemma2:9b
fi

read -p "Pull deepseek-coder for advanced code analysis? [y/N]: " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "3. Pulling deepseek-coder:6.7b..."
    ollama pull deepseek-coder:6.7b
fi

# Install Python dependencies
echo ""
echo "Installing Python dependencies..."

# Check if Python is available
if ! command -v python3 &> /dev/null; then
    echo "❌ Python 3 not found. Please install Python 3.8 or later."
    exit 1
fi

echo "✓ Python 3 is installed"

# Install requests library
python3 -m pip install --user requests toml

echo ""
echo "✅ Setup complete!"
echo ""
echo "Available models:"
ollama list
echo ""
echo "Usage examples:"
echo "  # Scan project for issues"
echo "  python3 tools/ollama_assistant.py --scan"
echo ""
echo "  # Auto-fix detected issues"
echo "  python3 tools/ollama_assistant.py --fix"
echo ""
echo "  # Watch mode (continuous monitoring)"
echo "  python3 tools/ollama_assistant.py --watch"
echo ""
echo "  # Analyze git diff"
echo "  python3 tools/ollama_assistant.py --diff"
echo ""
echo "For more options: python3 tools/ollama_assistant.py --help"
