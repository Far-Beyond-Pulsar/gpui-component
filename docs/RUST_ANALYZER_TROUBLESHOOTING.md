# Rust Analyzer Troubleshooting Guide

## Common Issues and Solutions

### ❌ Issue: "rust-analyzer exited unexpectedly (status: 1)"

This means rust-analyzer started but immediately crashed. Here are the common causes:

---

## 1. **Rust-Analyzer Not Installed**

### Symptoms:
```
⚠️ rust-analyzer check failed: Unknown binary 'rust-analyzer.exe' in official toolchain
❌ rust-analyzer exited with status: ExitStatus(ExitStatus(1))
```

### Solution:
Install rust-analyzer as a rustup component:
```bash
rustup component add rust-analyzer
```

### Verify:
```bash
rust-analyzer --version
# Should show: rust-analyzer 1.89.0 (or similar)
```

---

## 2. **Wrong rust-analyzer Binary**

### Symptoms:
```
⚠️ Could not run rust-analyzer.exe: The system cannot find the file specified
```

### Solution:
The engine checks these locations in order:
1. System PATH
2. `%CARGO_HOME%\bin\` (or `$CARGO_HOME/bin/`)
3. `~/.cargo/bin/`
4. `%USERPROFILE%\.cargo\bin\` (Windows)

Make sure rust-analyzer is installed in one of these locations.

### Manual Installation:
If rustup doesn't work, download from: https://rust-analyzer.github.io/

---

## 3. **Invalid Workspace**

### Symptoms:
```
rust-analyzer stderr: error: failed to load workspace
rust-analyzer stderr: caused by: could not find Cargo.toml
```

### Solution:
Ensure your project has:
- `Cargo.toml` in the root
- Valid Rust project structure
- At least one `src/` directory

### Check:
```bash
cd your_project
ls Cargo.toml    # Should exist
cargo check      # Should work
```

---

## 4. **Corrupted Rust Analyzer Cache**

### Symptoms:
```
rust-analyzer stderr: error: failed to load cached data
❌ rust-analyzer exited unexpectedly
```

### Solution:
Delete the rust-analyzer cache:

**Windows:**
```powershell
Remove-Item -Recurse -Force "$env:USERPROFILE\AppData\Local\rust-analyzer"
```

**Linux/Mac:**
```bash
rm -rf ~/.cache/rust-analyzer
```

Then restart the engine.

---

## 5. **Missing Dependencies**

### Symptoms:
```
rust-analyzer stderr: error: failed to resolve dependencies
```

### Solution:
Run cargo in your project to fetch dependencies:
```bash
cd your_project
cargo fetch
cargo build
```

---

## 6. **Incompatible Rust Version**

### Symptoms:
```
rust-analyzer stderr: error: unsupported Rust version
```

### Solution:
Update Rust:
```bash
rustup update stable
rustup component add rust-analyzer
```

---

## 7. **Process Permission Issues**

### Symptoms:
```
❌ Failed to spawn rust-analyzer: Permission denied
```

### Solution:

**Linux/Mac:**
```bash
chmod +x ~/.cargo/bin/rust-analyzer
```

**Windows:**
- Check antivirus isn't blocking it
- Run engine as administrator (if needed)

---

## How to Diagnose

### 1. Check if rust-analyzer works standalone:
```bash
cd your_project
rust-analyzer --version
```

### 2. Test rust-analyzer manually:
```bash
cd your_project
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":"file:///path/to/project"}}' | rust-analyzer
```

### 3. Check the Pulsar Engine console output:
Look for lines starting with:
- `rust-analyzer stderr:` - Actual error messages
- `❌ rust-analyzer` - Critical errors
- `⚠️` - Warnings

### 4. Check project structure:
```bash
cd your_project
tree -L 2
# Should show:
# ├── Cargo.toml
# ├── src/
# │   └── lib.rs or main.rs
```

---

## Expected Behavior

### Successful Startup:
```
🔧 Rust Analyzer Manager initialized
   Using: "rust-analyzer.exe"
   Version: rust-analyzer 1.89.0
🚀 Starting rust-analyzer for: "C:\path\to\project"
✓ rust-analyzer process spawned (PID: 12345)
✓ Sent initialize request to rust-analyzer
[Footer: 🟡 Starting... → 🟡 Indexing (0%) → 🟡 Indexing (45%) → 🟢 Ready ✓]
```

### With Issues:
```
⚠️ rust-analyzer check failed: Unknown binary 'rust-analyzer.exe'
❌ rust-analyzer is not installed!
   Install it with: rustup component add rust-analyzer
```

---

## Footer Status Meanings

| Icon | Status | Meaning |
|------|--------|---------|
| ⚪ Gray | Idle | Not started yet |
| 🟡 Yellow | Starting | Process spawning |
| 🟡 Yellow | Indexing (X%) | Analyzing code |
| 🟢 Green | Ready ✓ | Fully operational |
| 🔴 Red | Error | Something failed |
| ⚪ Gray | Stopped | Manually stopped |

---

## Actions When Errors Occur

### 1. Check Console Logs
All rust-analyzer stderr output is logged to the console.

### 2. Try Restarting
Click the **↻ Restart** button in the footer.

### 3. Stop and Check
1. Click **❌ Stop** button
2. Run rust-analyzer manually to diagnose
3. Fix the issue
4. Click **▶ Start** button

### 4. Verify Installation
```bash
rustup component list | grep rust-analyzer
# Should show: rust-analyzer (installed)
```

### 5. Re-install if Needed
```bash
rustup component remove rust-analyzer
rustup component add rust-analyzer
```

---

## Platform-Specific Notes

### Windows
- Uses `rust-analyzer.exe`
- Checks `%USERPROFILE%\.cargo\bin\`
- May need to add to PATH manually

### Linux/Mac
- Uses `rust-analyzer` (no extension)
- Checks `~/.cargo/bin/`
- May need execute permissions

---

## Still Not Working?

### Create a Minimal Test Case:
```bash
cargo new test_project
cd test_project
rust-analyzer --version  # Verify works
```

Then open `test_project` in Pulsar Engine.

If it works with the minimal project but not your project:
- Issue is with your project structure
- Check for corrupted `Cargo.lock`
- Try `cargo clean` in your project

If it doesn't work even with minimal project:
- rust-analyzer is not properly installed
- Follow installation steps again
- Check PATH settings

---

## Getting Help

When reporting issues, include:
1. **Console output** - All messages starting with `rust-analyzer`
2. **Version info:**
   ```bash
   rust-analyzer --version
   rustc --version
   ```
3. **Project structure:**
   ```bash
   ls -la
   cat Cargo.toml
   ```
4. **Footer status** - What the footer shows

---

## Quick Fix Checklist

- [ ] rust-analyzer installed? → `rustup component add rust-analyzer`
- [ ] Version works? → `rust-analyzer --version`
- [ ] Cargo.toml exists? → Check project root
- [ ] Dependencies fetched? → `cargo fetch`
- [ ] Cache cleared? → Delete ~/.cache/rust-analyzer
- [ ] Restarted engine? → Try restart button
- [ ] Manual test works? → Run rust-analyzer in terminal

---

## Summary

**Most Common Issue:** rust-analyzer not installed
**Quick Fix:** `rustup component add rust-analyzer`
**Verify:** `rust-analyzer --version`
**Restart:** Click ↻ button in footer

The engine now provides detailed error messages in the console, so check there first when issues occur!
