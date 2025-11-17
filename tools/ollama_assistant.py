#!/usr/bin/env python3
"""
Ollama AI Coding Assistant for rusb

This tool uses Ollama with Gemma 3 to automatically find and fix errors,
format code, run static analysis, and provide intelligent code assistance
for the rusb USB library project.

Features:
- Automatic error detection and fixing
- Code formatting (rustfmt, clippy)
- Integration with rust-analyzer
- Git diff analysis
- Project-wide code scanning
- Autonomous code improvement

Usage:
    python ollama_assistant.py --scan          # Scan project for issues
    python ollama_assistant.py --fix           # Auto-fix detected issues
    python ollama_assistant.py --analyze FILE  # Analyze specific file
    python ollama_assistant.py --watch         # Watch mode (continuous)
"""

import argparse
import json
import os
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Tuple
import time
import re
import logging
from datetime import datetime

try:
    import requests
except ImportError:
    print("Error: requests library not found. Install with: pip install requests")
    sys.exit(1)

try:
    import toml
except ImportError:
    print("Error: toml library not found. Install with: pip install toml")
    sys.exit(1)


@dataclass
class Issue:
    """Represents a code issue found by analysis"""
    file_path: str
    line_number: int
    severity: str  # "error", "warning", "suggestion"
    message: str
    tool: str  # "clippy", "rustc", "rust-analyzer", etc.
    suggestion: Optional[str] = None


@dataclass
class Fix:
    """Represents a fix to be applied"""
    file_path: str
    original_content: str
    fixed_content: str
    description: str


@dataclass
class Config:
    """Configuration loaded from ollama_config.toml"""
    # Ollama settings
    ollama_base_url: str = "http://localhost:11434"
    ollama_model: str = "gemma2:2b"
    ollama_temperature: float = 0.1
    ollama_max_tokens: int = 2048

    # Analysis settings
    enabled_tools: List[str] = field(default_factory=lambda: ["clippy", "rustc", "rustfmt"])
    auto_fix_severity: List[str] = field(default_factory=lambda: ["error", "warning"])
    include_patterns: List[str] = field(default_factory=lambda: ["*.rs"])
    exclude_patterns: List[str] = field(default_factory=lambda: ["*/target/*", "*/test_data/*"])

    # Watch mode settings
    watch_interval: int = 5
    watch_auto_fix: bool = False
    watch_run_tests: bool = True

    # Git settings
    git_pre_commit_check: bool = True
    git_max_diff_size: int = 5000

    # Formatting settings
    auto_format: bool = True
    rustfmt_edition: str = "2024"

    # Clippy settings
    clippy_deny: List[str] = field(default_factory=lambda: ["warnings"])
    clippy_allow: List[str] = field(default_factory=list)

    # Fix settings
    create_backup: bool = True
    backup_dir: str = ".ollama_backups"
    max_batch_size: int = 10

    # Logging settings
    log_level: str = "info"
    log_file: str = "tools/ollama_assistant.log"
    log_verbose: bool = False

    @classmethod
    def load_from_file(cls, config_path: Path) -> 'Config':
        """Load configuration from TOML file"""
        if not config_path.exists():
            logging.warning(f"Config file not found: {config_path}, using defaults")
            return cls()

        try:
            data = toml.load(config_path)

            return cls(
                # Ollama settings
                ollama_base_url=data.get("ollama", {}).get("base_url", "http://localhost:11434"),
                ollama_model=data.get("ollama", {}).get("model", "gemma2:2b"),
                ollama_temperature=data.get("ollama", {}).get("temperature", 0.1),
                ollama_max_tokens=data.get("ollama", {}).get("max_tokens", 2048),

                # Analysis settings
                enabled_tools=data.get("analysis", {}).get("enabled_tools", ["clippy", "rustc", "rustfmt"]),
                auto_fix_severity=data.get("analysis", {}).get("auto_fix_severity", ["error", "warning"]),
                include_patterns=data.get("analysis", {}).get("include_patterns", ["*.rs"]),
                exclude_patterns=data.get("analysis", {}).get("exclude_patterns", ["*/target/*", "*/test_data/*"]),

                # Watch mode settings
                watch_interval=data.get("watch_mode", {}).get("interval", 5),
                watch_auto_fix=data.get("watch_mode", {}).get("auto_fix", False),
                watch_run_tests=data.get("watch_mode", {}).get("run_tests", True),

                # Git settings
                git_pre_commit_check=data.get("git", {}).get("pre_commit_check", True),
                git_max_diff_size=data.get("git", {}).get("max_diff_size", 5000),

                # Formatting settings
                auto_format=data.get("formatting", {}).get("auto_format", True),
                rustfmt_edition=data.get("formatting", {}).get("edition", "2024"),

                # Clippy settings
                clippy_deny=data.get("clippy", {}).get("deny", ["warnings"]),
                clippy_allow=data.get("clippy", {}).get("allow", []),

                # Fix settings
                create_backup=data.get("fixes", {}).get("create_backup", True),
                backup_dir=data.get("fixes", {}).get("backup_dir", ".ollama_backups"),
                max_batch_size=data.get("fixes", {}).get("max_batch_size", 10),

                # Logging settings
                log_level=data.get("logging", {}).get("level", "info"),
                log_file=data.get("logging", {}).get("file", "tools/ollama_assistant.log"),
                log_verbose=data.get("logging", {}).get("verbose", False),
            )
        except Exception as e:
            logging.error(f"Error loading config file: {e}")
            logging.warning("Using default configuration")
            return cls()


def setup_logging(config: Config) -> None:
    """Setup logging based on configuration"""
    log_level = getattr(logging, config.log_level.upper(), logging.INFO)

    # Create log file directory if needed
    log_path = Path(config.log_file)
    log_path.parent.mkdir(parents=True, exist_ok=True)

    # Configure logging
    logging.basicConfig(
        level=log_level,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
        handlers=[
            logging.FileHandler(config.log_file),
            logging.StreamHandler(sys.stdout) if config.log_verbose else logging.NullHandler()
        ]
    )


class OllamaClient:
    """Client for communicating with Ollama API"""

    def __init__(self, config: Config):
        self.model = config.ollama_model
        self.base_url = config.ollama_base_url
        self.temperature = config.ollama_temperature
        self.max_tokens = config.ollama_max_tokens
        self.conversation_history = []
        self.logger = logging.getLogger(__name__)

    def check_connection(self) -> bool:
        """Check if Ollama is running and model is available"""
        try:
            response = requests.get(f"{self.base_url}/api/tags", timeout=5)
            if response.status_code == 200:
                models = response.json().get("models", [])
                model_names = [m["name"] for m in models]
                if any(self.model in name for name in model_names):
                    self.logger.info(f"Connected to Ollama, model: {self.model}")
                    return True
                else:
                    self.logger.warning(f"Model {self.model} not found. Available models: {model_names}")
                    print(f"Warning: Model {self.model} not found. Available models: {model_names}")
                    print(f"Pull the model with: ollama pull {self.model}")
                    return False
            return False
        except requests.exceptions.RequestException as e:
            self.logger.error(f"Cannot connect to Ollama at {self.base_url}: {e}")
            print(f"Error: Cannot connect to Ollama at {self.base_url}")
            print(f"Make sure Ollama is running: {e}")
            return False

    def generate(self, prompt: str, system_prompt: Optional[str] = None) -> str:
        """Generate response from Ollama"""
        messages = []

        if system_prompt:
            messages.append({"role": "system", "content": system_prompt})

        # Add conversation history
        messages.extend(self.conversation_history)

        # Add current prompt
        messages.append({"role": "user", "content": prompt})

        try:
            response = requests.post(
                f"{self.base_url}/api/chat",
                json={
                    "model": self.model,
                    "messages": messages,
                    "stream": False,
                    "options": {
                        "temperature": self.temperature,
                        "num_predict": self.max_tokens,
                    }
                },
                timeout=120
            )

            if response.status_code == 200:
                result = response.json()
                assistant_message = result["message"]["content"]

                # Update conversation history
                self.conversation_history.append({"role": "user", "content": prompt})
                self.conversation_history.append({"role": "assistant", "content": assistant_message})

                # Keep only last 10 exchanges to avoid context bloat
                if len(self.conversation_history) > 20:
                    self.conversation_history = self.conversation_history[-20:]

                self.logger.debug(f"Generated response ({len(assistant_message)} chars)")
                return assistant_message
            else:
                error_msg = f"Ollama returned status {response.status_code}"
                self.logger.error(error_msg)
                print(f"Error: {error_msg}")
                return ""

        except requests.exceptions.RequestException as e:
            self.logger.error(f"Error communicating with Ollama: {e}")
            print(f"Error communicating with Ollama: {e}")
            return ""

    def reset_conversation(self):
        """Clear conversation history"""
        self.conversation_history = []


class RustAnalyzer:
    """Wrapper for Rust development tools"""

    def __init__(self, project_root: Path, config: Config):
        self.project_root = project_root
        self.rusb_path = project_root / "rusb"
        self.config = config
        self.logger = logging.getLogger(__name__)

    def run_clippy(self, file_path: Optional[Path] = None) -> List[Issue]:
        """Run cargo clippy and parse results"""
        issues = []

        try:
            cmd = ["cargo", "clippy", "--message-format=json", "--all-targets"]
            if file_path:
                # Clippy doesn't support single file, run on whole project
                pass

            result = subprocess.run(
                cmd,
                cwd=self.rusb_path,
                capture_output=True,
                text=True,
                timeout=120
            )

            # Parse JSON output
            for line in result.stdout.splitlines():
                if line.strip():
                    try:
                        data = json.loads(line)
                        if data.get("reason") == "compiler-message":
                            message = data.get("message", {})
                            if message.get("spans"):
                                span = message["spans"][0]
                                issue = Issue(
                                    file_path=span.get("file_name", ""),
                                    line_number=span.get("line_start", 0),
                                    severity=message.get("level", "warning"),
                                    message=message.get("message", ""),
                                    tool="clippy",
                                    suggestion=message.get("rendered")
                                )
                                issues.append(issue)
                    except json.JSONDecodeError:
                        continue

        except subprocess.TimeoutExpired:
            print("Warning: clippy timed out")
        except Exception as e:
            print(f"Error running clippy: {e}")

        return issues

    def run_rustfmt(self, file_path: Optional[Path] = None, check_only: bool = False) -> bool:
        """Run rustfmt on file or project"""
        try:
            cmd = ["cargo", "fmt"]
            if check_only:
                cmd.append("--check")
            if file_path:
                cmd.extend(["--", str(file_path)])

            result = subprocess.run(
                cmd,
                cwd=self.rusb_path,
                capture_output=True,
                text=True,
                timeout=30
            )

            return result.returncode == 0

        except Exception as e:
            print(f"Error running rustfmt: {e}")
            return False

    def compile_check(self, file_path: Optional[Path] = None) -> List[Issue]:
        """Run cargo check and parse errors"""
        issues = []

        try:
            cmd = ["cargo", "check", "--message-format=json"]

            result = subprocess.run(
                cmd,
                cwd=self.rusb_path,
                capture_output=True,
                text=True,
                timeout=120
            )

            # Parse JSON output
            for line in result.stdout.splitlines():
                if line.strip():
                    try:
                        data = json.loads(line)
                        if data.get("reason") == "compiler-message":
                            message = data.get("message", {})
                            if message.get("spans"):
                                span = message["spans"][0]
                                issue = Issue(
                                    file_path=span.get("file_name", ""),
                                    line_number=span.get("line_start", 0),
                                    severity=message.get("level", "error"),
                                    message=message.get("message", ""),
                                    tool="rustc",
                                    suggestion=message.get("rendered")
                                )
                                issues.append(issue)
                    except json.JSONDecodeError:
                        continue

        except Exception as e:
            print(f"Error running cargo check: {e}")

        return issues

    def get_git_diff(self, staged: bool = False) -> str:
        """Get git diff output"""
        try:
            cmd = ["git", "diff"]
            if staged:
                cmd.append("--staged")

            result = subprocess.run(
                cmd,
                cwd=self.project_root,
                capture_output=True,
                text=True,
                timeout=10
            )

            return result.stdout

        except Exception as e:
            print(f"Error getting git diff: {e}")
            return ""

    def scan_directory(self, pattern: str = "*.rs") -> List[Path]:
        """Scan for Rust files in project"""
        rust_files = []
        for file in self.rusb_path.rglob(pattern):
            if "target" not in str(file):  # Skip build artifacts
                rust_files.append(file)
        return rust_files


class CodeAssistant:
    """AI-powered code assistant using Ollama"""

    def __init__(self, project_root: Path, config: Config):
        self.project_root = project_root
        self.config = config
        self.ollama = OllamaClient(config)
        self.analyzer = RustAnalyzer(project_root, config)
        self.logger = logging.getLogger(__name__)

        self.system_prompt = """You are an expert Rust developer and code reviewer specializing in USB device drivers and systems programming.

Your task is to analyze Rust code for the rusb library (a safe Rust wrapper around libusb) and provide:
1. Clear, actionable fixes for errors and warnings
2. Code improvements following Rust best practices
3. Safety and correctness analysis
4. Performance optimization suggestions

When providing fixes:
- Output ONLY the corrected code, no explanations (unless asked)
- Maintain the existing code style and structure
- Preserve all functionality
- Add comments only where clarity is needed
- Follow Rust naming conventions and idioms
- Ensure memory safety and proper error handling

The rusb project structure:
- rusb/src/lib.rs - Public API
- rusb/src/platform/ - Platform-specific backends (Linux, Windows, macOS, WebUSB)
- rusb/src/transfer/ - USB transfer types
- rusb/src/hotplug/ - Hotplug detection
- rusb/tests/ - Test suite

Focus on:
- Correct unsafe code usage
- Proper error propagation
- Platform abstraction
- Type safety
- Zero-cost abstractions"""

    def initialize(self) -> bool:
        """Initialize and check connectivity"""
        print("Initializing Ollama AI Assistant...")
        if not self.ollama.check_connection():
            return False
        print(f"✓ Connected to Ollama (model: {self.ollama.model})")

        # Create backup directory if needed
        if self.config.create_backup:
            backup_dir = self.project_root / self.config.backup_dir
            backup_dir.mkdir(parents=True, exist_ok=True)
            self.logger.info(f"Backup directory: {backup_dir}")

        return True

    def create_backup(self, file_path: str) -> Optional[Path]:
        """Create a timestamped backup of a file"""
        if not self.config.create_backup:
            return None

        try:
            source_path = Path(file_path)
            if not source_path.exists():
                self.logger.warning(f"Cannot backup non-existent file: {file_path}")
                return None

            # Create backup directory structure
            backup_root = self.project_root / self.config.backup_dir
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")

            # Preserve directory structure in backup
            rel_path = source_path.relative_to(self.project_root)
            backup_path = backup_root / timestamp / rel_path

            # Create parent directories
            backup_path.parent.mkdir(parents=True, exist_ok=True)

            # Copy file
            import shutil
            shutil.copy2(source_path, backup_path)

            self.logger.info(f"Created backup: {backup_path}")
            print(f"   📦 Backup: {backup_path}")

            return backup_path

        except Exception as e:
            self.logger.error(f"Failed to create backup: {e}")
            print(f"   ⚠️  Backup failed: {e}")
            return None

    def rollback(self, backup_path: Path, target_path: Path) -> bool:
        """Restore a file from backup"""
        try:
            import shutil
            shutil.copy2(backup_path, target_path)
            self.logger.info(f"Restored {target_path} from {backup_path}")
            print(f"✓ Restored: {target_path}")
            return True
        except Exception as e:
            self.logger.error(f"Rollback failed: {e}")
            print(f"✗ Rollback failed: {e}")
            return False

    def list_backups(self) -> List[Tuple[str, List[Path]]]:
        """List all available backups grouped by timestamp"""
        backup_root = self.project_root / self.config.backup_dir

        if not backup_root.exists():
            print("No backups found")
            return []

        backups = []
        for timestamp_dir in sorted(backup_root.iterdir(), reverse=True):
            if timestamp_dir.is_dir():
                files = list(timestamp_dir.rglob('*'))
                files = [f for f in files if f.is_file()]
                backups.append((timestamp_dir.name, files))

        return backups

    def restore_backup(self, timestamp: str) -> bool:
        """Restore all files from a specific backup"""
        backup_root = self.project_root / self.config.backup_dir / timestamp

        if not backup_root.exists():
            print(f"❌ Backup not found: {timestamp}")
            return False

        print(f"📦 Restoring backup from {timestamp}...")

        restored_count = 0
        for backup_file in backup_root.rglob('*'):
            if not backup_file.is_file():
                continue

            # Calculate target path
            rel_path = backup_file.relative_to(backup_root)
            target_path = self.project_root / rel_path

            try:
                import shutil
                target_path.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(backup_file, target_path)
                print(f"   ✓ Restored: {rel_path}")
                restored_count += 1
            except Exception as e:
                print(f"   ✗ Failed to restore {rel_path}: {e}")
                self.logger.error(f"Failed to restore {rel_path}: {e}")

        print(f"\n✓ Restored {restored_count} file(s) from backup")
        return True

    def atomic_write(self, file_path: Path, content: str) -> bool:
        """Write file atomically to prevent corruption"""
        import tempfile
        import shutil

        try:
            # Ensure parent directory exists
            file_path.parent.mkdir(parents=True, exist_ok=True)

            # Write to temporary file in the same directory
            # (same directory ensures same filesystem for atomic move)
            with tempfile.NamedTemporaryFile(
                mode='w',
                encoding='utf-8',
                dir=file_path.parent,
                delete=False,
                suffix='.tmp'
            ) as tmp_file:
                tmp_file.write(content)
                tmp_path = Path(tmp_file.name)

            # Atomic replace (on Windows, need to remove target first)
            if sys.platform == 'win32':
                if file_path.exists():
                    file_path.unlink()

            shutil.move(str(tmp_path), str(file_path))

            self.logger.debug(f"Atomic write successful: {file_path}")
            return True

        except Exception as e:
            self.logger.error(f"Atomic write failed for {file_path}: {e}")
            # Clean up temp file if it exists
            if 'tmp_path' in locals() and tmp_path.exists():
                try:
                    tmp_path.unlink()
                except:
                    pass
            return False

    def validate_rust_syntax(self, file_path: Path) -> bool:
        """Validate Rust file syntax using cargo check"""
        try:
            result = subprocess.run(
                ["cargo", "check", "--message-format=json"],
                cwd=self.analyzer.rusb_path,
                capture_output=True,
                text=True,
                timeout=30
            )

            # Check if the specific file has errors
            for line in result.stdout.splitlines():
                if not line.strip():
                    continue
                try:
                    data = json.loads(line)
                    if data.get("reason") == "compiler-message":
                        message = data.get("message", {})
                        if message.get("level") == "error":
                            spans = message.get("spans", [])
                            for span in spans:
                                if Path(span.get("file_name", "")) == file_path:
                                    self.logger.error(f"Syntax error in {file_path}: {message.get('message')}")
                                    return False
                except json.JSONDecodeError:
                    continue

            self.logger.debug(f"Syntax validation passed: {file_path}")
            return True

        except subprocess.TimeoutExpired:
            self.logger.warning(f"Syntax validation timed out for {file_path}")
            return True  # Don't fail on timeout, assume OK
        except Exception as e:
            self.logger.error(f"Syntax validation error: {e}")
            return True  # Don't fail on validation error, assume OK

    def analyze_issue(self, issue: Issue) -> Optional[Fix]:
        """Analyze an issue and generate a fix"""
        # Read the file
        try:
            file_path = self.project_root / "rusb" / issue.file_path
            if not file_path.exists():
                file_path = Path(issue.file_path)

            if not file_path.exists():
                print(f"Warning: File not found: {issue.file_path}")
                return None

            with open(file_path, 'r', encoding='utf-8') as f:
                original_content = f.read()

            # Extract context around the issue (±5 lines)
            lines = original_content.splitlines()
            start_line = max(0, issue.line_number - 6)
            end_line = min(len(lines), issue.line_number + 5)
            context_lines = lines[start_line:end_line]
            context = "\n".join(f"{i+start_line+1:4d} | {line}" for i, line in enumerate(context_lines))

            # Create prompt for Ollama
            prompt = f"""Fix this Rust code issue:

File: {issue.file_path}
Line: {issue.line_number}
Severity: {issue.severity}
Tool: {issue.tool}

Issue:
{issue.message}

Code context:
```rust
{context}
```

Provide the CORRECTED code for the entire affected section. Include surrounding context for proper integration.
Output only valid Rust code, no explanations."""

            print(f"\n🔍 Analyzing: {issue.file_path}:{issue.line_number}")
            print(f"   Issue: {issue.message}")

            # Get fix from Ollama
            response = self.ollama.generate(prompt, system_prompt=self.system_prompt)

            if not response:
                return None

            # Extract code from response (handle markdown code blocks)
            fixed_code = self._extract_code(response)

            if not fixed_code or fixed_code == context:
                print("   ⚠️  No fix generated")
                return None

            # Apply the fix to the full file content
            fixed_content = self._apply_fix(original_content, fixed_code, start_line, end_line)

            return Fix(
                file_path=str(file_path),
                original_content=original_content,
                fixed_content=fixed_content,
                description=f"Fix {issue.severity} in {issue.file_path}:{issue.line_number}"
            )

        except Exception as e:
            print(f"Error analyzing issue: {e}")
            return None

    def _extract_code(self, response: str) -> str:
        """Extract code from Ollama response (handle markdown)"""
        # Try to extract from ```rust code block
        code_block_pattern = r"```(?:rust)?\n(.*?)```"
        matches = re.findall(code_block_pattern, response, re.DOTALL)

        if matches:
            return matches[0].strip()

        # If no code block, return entire response
        return response.strip()

    def _apply_fix(self, original: str, fixed_section: str, start_line: int, end_line: int) -> str:
        """Apply fixed code section to original file"""
        lines = original.splitlines()

        # Replace the affected lines
        fixed_lines = fixed_section.splitlines()

        # Simple replacement strategy: replace the context lines
        new_lines = lines[:start_line] + fixed_lines + lines[end_line:]

        return "\n".join(new_lines)

    def scan_project(self) -> List[Issue]:
        """Scan entire project for issues"""
        print("\n🔎 Scanning project with Rust tools...")

        all_issues = []

        # Run clippy
        print("  Running cargo clippy...")
        clippy_issues = self.analyzer.run_clippy()
        all_issues.extend(clippy_issues)
        print(f"  Found {len(clippy_issues)} clippy issues")

        # Run cargo check
        print("  Running cargo check...")
        check_issues = self.analyzer.compile_check()
        all_issues.extend(check_issues)
        print(f"  Found {len(check_issues)} compilation issues")

        # Check formatting
        print("  Checking code formatting...")
        if not self.analyzer.run_rustfmt(check_only=True):
            print("  Code formatting issues detected (run cargo fmt)")

        return all_issues

    def auto_fix(self, issues: List[Issue], dry_run: bool = False) -> List[Fix]:
        """Automatically fix detected issues"""
        fixes = []

        # Group issues by file
        issues_by_file: Dict[str, List[Issue]] = {}
        for issue in issues:
            if issue.file_path not in issues_by_file:
                issues_by_file[issue.file_path] = []
            issues_by_file[issue.file_path].append(issue)

        print(f"\n🔧 Auto-fixing {len(issues)} issues in {len(issues_by_file)} files...")

        for file_path, file_issues in issues_by_file.items():
            # Sort by line number (fix from bottom to top to preserve line numbers)
            file_issues.sort(key=lambda x: x.line_number, reverse=True)

            for issue in file_issues:
                # Skip non-critical issues for now
                if issue.severity == "help":
                    continue

                fix = self.analyze_issue(issue)

                if fix:
                    if dry_run:
                        print(f"   ✓ Generated fix (dry-run)")
                    else:
                        # Create backup before applying fix
                        backup_path = self.create_backup(fix.file_path)

                        # Apply the fix using atomic write
                        fix_path = Path(fix.file_path)
                        if self.atomic_write(fix_path, fix.fixed_content):
                            # Validate the fix
                            if self.validate_rust_syntax(fix_path):
                                print(f"   ✓ Applied fix: {fix.description}")
                                fixes.append(fix)
                            else:
                                print(f"   ✗ Syntax validation failed")
                                self.logger.error(f"Syntax validation failed for {fix_path}")

                                # Rollback on validation failure
                                if backup_path:
                                    print(f"   🔄 Rolling back due to syntax error...")
                                    self.rollback(backup_path, fix_path)
                        else:
                            print(f"   ✗ Failed to write fix")

                            # Attempt rollback if backup exists
                            if backup_path:
                                print(f"   🔄 Attempting rollback...")
                                self.rollback(backup_path, fix_path)

        return fixes

    def analyze_git_diff(self) -> str:
        """Analyze recent git changes"""
        diff = self.analyzer.get_git_diff()

        if not diff:
            print("No git changes to analyze")
            return ""

        # Limit diff size based on config
        max_size = self.config.git_max_diff_size
        if len(diff) > max_size:
            diff = diff[:max_size]
            self.logger.info(f"Truncated diff to {max_size} chars")

        prompt = f"""Analyze this git diff for potential issues:

```diff
{diff}
```

Identify:
1. Potential bugs or logic errors
2. Code style violations
3. Unsafe patterns
4. Performance concerns
5. Missing error handling

Be concise and actionable."""

        print("\n📊 Analyzing git diff...")
        response = self.ollama.generate(prompt, system_prompt=self.system_prompt)

        return response

    def watch_mode(self):
        """Continuous monitoring and fixing"""
        interval = self.config.watch_interval
        auto_fix = self.config.watch_auto_fix
        run_tests = self.config.watch_run_tests

        print(f"\n👁️  Watch mode activated (checking every {interval}s)")
        if auto_fix:
            print("   Auto-fix: ENABLED")
        else:
            print("   Auto-fix: Prompt for confirmation")
        print("Press Ctrl+C to stop\n")

        try:
            last_issues_count = 0

            while True:
                issues = self.scan_project()

                if len(issues) != last_issues_count:
                    if len(issues) > 0:
                        print(f"\n⚠️  {len(issues)} issues detected")

                        # Auto-fix based on config
                        should_fix = auto_fix or input("Auto-fix issues? [y/N]: ").lower() == 'y'

                        if should_fix:
                            fixes = self.auto_fix(issues, dry_run=False)

                            # Run tests if configured
                            if run_tests and fixes:
                                print("\nRunning tests...")
                                try:
                                    result = subprocess.run(
                                        ["cargo", "test"],
                                        cwd=self.analyzer.rusb_path,
                                        capture_output=True,
                                        timeout=300
                                    )
                                    if result.returncode == 0:
                                        print("✓ Tests passed")
                                    else:
                                        print("✗ Tests failed")
                                        self.logger.warning("Tests failed after applying fixes")
                                except Exception as e:
                                    print(f"Error running tests: {e}")
                    else:
                        print("\n✓ No issues found")

                    last_issues_count = len(issues)

                time.sleep(interval)

        except KeyboardInterrupt:
            print("\n\nWatch mode stopped")


def main():
    parser = argparse.ArgumentParser(
        description="Ollama AI Coding Assistant for rusb",
        formatter_class=argparse.RawDescriptionHelpFormatter
    )

    parser.add_argument("--scan", action="store_true",
                        help="Scan project for issues")
    parser.add_argument("--fix", action="store_true",
                        help="Auto-fix detected issues")
    parser.add_argument("--dry-run", action="store_true",
                        help="Dry run (don't apply fixes)")
    parser.add_argument("--analyze", metavar="FILE",
                        help="Analyze specific file")
    parser.add_argument("--watch", action="store_true",
                        help="Watch mode (continuous monitoring)")
    parser.add_argument("--diff", action="store_true",
                        help="Analyze git diff")
    parser.add_argument("--list-backups", action="store_true",
                        help="List available backups")
    parser.add_argument("--undo", metavar="TIMESTAMP",
                        help="Restore files from backup by timestamp (YYYYMMDD_HHMMSS)")
    parser.add_argument("--model", default=None,
                        help="Ollama model to use (overrides config)")
    parser.add_argument("--interval", type=int, default=None,
                        help="Watch mode interval in seconds (overrides config)")
    parser.add_argument("--config", default="tools/ollama_config.toml",
                        help="Path to config file (default: tools/ollama_config.toml)")

    args = parser.parse_args()

    # Find project root
    script_dir = Path(__file__).parent
    project_root = script_dir.parent

    # Load configuration
    config_path = project_root / args.config
    config = Config.load_from_file(config_path)

    # Override config with command-line arguments
    if args.model:
        config.ollama_model = args.model
    if args.interval:
        config.watch_interval = args.interval

    # Setup logging
    setup_logging(config)

    # Initialize assistant
    assistant = CodeAssistant(project_root, config)

    if not assistant.initialize():
        print("\n❌ Failed to initialize Ollama assistant")
        print("\nMake sure Ollama is running:")
        print("  1. Install Ollama from https://ollama.ai")
        print(f"  2. Run: ollama pull {config.ollama_model}")
        print("  3. Verify: ollama list")
        sys.exit(1)

    # Execute command
    if args.list_backups:
        backups = assistant.list_backups()
        if backups:
            print("\n📦 Available Backups:\n")
            for timestamp, files in backups:
                print(f"Timestamp: {timestamp}")
                print(f"  Files: {len(files)}")
                for file in files[:5]:  # Show first 5 files
                    rel_path = file.relative_to(project_root / config.backup_dir / timestamp)
                    print(f"    - {rel_path}")
                if len(files) > 5:
                    print(f"    ... and {len(files) - 5} more")
                print()

    elif args.undo:
        assistant.restore_backup(args.undo)

    elif args.scan:
        issues = assistant.scan_project()

        if issues:
            print(f"\n📋 Found {len(issues)} issues:\n")
            for i, issue in enumerate(issues, 1):
                print(f"{i}. [{issue.severity.upper()}] {issue.file_path}:{issue.line_number}")
                print(f"   {issue.message}\n")
        else:
            print("\n✓ No issues found!")

    elif args.fix:
        issues = assistant.scan_project()

        if issues:
            fixes = assistant.auto_fix(issues, dry_run=args.dry_run)
            print(f"\n✓ Applied {len(fixes)} fixes")

            if not args.dry_run:
                print("\nRun 'cargo test' to verify fixes")
        else:
            print("\n✓ No issues to fix!")

    elif args.analyze:
        file_path = Path(args.analyze)
        if not file_path.exists():
            print(f"Error: File not found: {file_path}")
            sys.exit(1)

        # Create a dummy issue to analyze the file
        issue = Issue(
            file_path=str(file_path),
            line_number=1,
            severity="analysis",
            message="Full file analysis requested",
            tool="manual"
        )

        fix = assistant.analyze_issue(issue)
        if fix:
            print(f"\n✓ Analysis complete")
            if args.dry_run:
                print("\nSuggested improvements:")
                print(fix.fixed_content)

    elif args.diff:
        analysis = assistant.analyze_git_diff()
        if analysis:
            print("\n📊 Git Diff Analysis:\n")
            print(analysis)

    elif args.watch:
        assistant.watch_mode()

    else:
        parser.print_help()


if __name__ == "__main__":
    main()
