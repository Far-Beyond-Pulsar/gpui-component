# Debugging File Sync Issues 🔍

## Problem: Diff Detection Not Working

If the file sync is not detecting differences or showing diffs, here's how to debug it.

---

## Enhanced Logging Added ✅

I've added comprehensive logging throughout the sync process. Look for these log messages:

### On HOST (when creating session):
```
HOST: Creating file manifest for <path>
HOST: Created manifest with X files
HOST: Serialized manifest to X bytes
HOST: Sent file manifest
```

### On JOINER (when joining session):
```
JOIN_SESSION: Sent RequestFileManifest
JOIN_SESSION: Received file manifest from <peer> (X bytes)
JOIN_SESSION: Parsed manifest with X files
SIMPLE_SYNC: Computing diff...
SIMPLE_SYNC: Diff computed: X to add, Y to update, Z to delete
JOIN_SESSION: Changes detected, showing sync UI
```

### In simple_sync module:
```
SIMPLE_SYNC: Creating manifest for <path>
SIMPLE_SYNC: Added file: <filename> (hash: <hash>, size: <size>)
SIMPLE_SYNC: Created manifest with X files
SIMPLE_SYNC: Sample files: [...]
SIMPLE_SYNC: Computing diff against remote manifest (X files)
SIMPLE_SYNC: Remote sample files: [...]
SIMPLE_SYNC: Local manifest has X files
SIMPLE_SYNC: File to ADD: <filename>
SIMPLE_SYNC: File to UPDATE: <filename> (local: <hash>, remote: <hash>)
SIMPLE_SYNC: Files to DELETE: [...]
```

---

## How to Debug

### Step 1: Enable Logging

Run with `RUST_LOG=info` or `RUST_LOG=debug` for more detail:

```bash
# Windows PowerShell
$env:RUST_LOG="debug"
cargo run

# Or for specific modules only
$env:RUST_LOG="pulsar_engine::ui=debug,simple_sync=debug"
cargo run
```

### Step 2: Check Logs

Look for these specific issues in the logs:

#### Issue 1: No manifest being created
**Symptom**: You don't see "Created manifest with X files"

**Possible causes**:
- Project root is not set
- All files are being ignored
- Permission issues reading files

**Fix**: Check the project_root path is correct

#### Issue 2: Manifest created but not sent
**Symptom**: You see "Created manifest" but not "Sent file manifest"

**Possible causes**:
- Serialization failed
- WebSocket connection dropped
- peer_id is None

**Fix**: Check WebSocket connection is stable

#### Issue 3: Manifest received but no diff
**Symptom**: You see "Received file manifest" but no "Computed diff"

**Possible causes**:
- Manifest parsing failed
- project_root is None
- Exception in compute_diff

**Fix**: Check for error logs after "Received file manifest"

#### Issue 4: Diff computed but shows "0 to add, 0 to update, 0 to delete"
**Symptom**: Diff is empty even though files are different

**Possible causes**:
- **Path separator mismatch** (Windows `\` vs Unix `/`)
- Files are identical (hashes match)
- Manifest comparison bug

**Fix**: Check the "Sample files" in logs to see path format

---

## Common Issues & Fixes

### Issue 1: Path Separator Mismatch ✅ FIXED

**Problem**: Windows uses `\` in paths, but comparison expects `/`

**Fix Applied**: All paths are now normalized to use forward slashes:
```rust
let normalized_path = relative_path
    .to_string_lossy()
    .replace('\\', "/");
```

### Issue 2: No Project Root

**Problem**: `project_root` is `None` when trying to create manifest

**Fix**: Ensure you have a project open before creating/joining session

**Code to check**:
```rust
// In state.rs initialization
let project_root = std::env::current_dir().ok();
```

### Issue 3: All Files Ignored

**Problem**: Every file matches an ignore pattern

**Fix**: Check your ignore patterns in `simple_sync.rs`:
```rust
const DEFAULT_IGNORES: &[&str] = &[
    ".git", "target", "dist", "build",
    "node_modules", ".vscode", ".idea",
    "*.exe", "*.dll", "*.log",
];
```

If your project only has ignored files, you won't see any diff!

---

## Testing the Fix

### Test 1: Basic Diff Detection

**Setup**:
1. Create a test project with a few files
2. Host creates session
3. Joiner joins with empty directory

**Expected**:
```
SIMPLE_SYNC: Diff computed: 5 to add, 0 to update, 0 to delete
```

**UI should show**:
- "5 files to add"
- List of files
- Sync button

### Test 2: Modified Files

**Setup**:
1. Both have same project
2. Host modifies 2 files
3. Joiner joins

**Expected**:
```
SIMPLE_SYNC: Diff computed: 0 to add, 2 to update, 0 to delete
SIMPLE_SYNC: File to UPDATE: file1.txt (local: abc123, remote: def456)
SIMPLE_SYNC: File to UPDATE: file2.txt (local: 123abc, remote: 456def)
```

### Test 3: Deleted Files

**Setup**:
1. Joiner has 3 files that host doesn't
2. Joiner joins

**Expected**:
```
SIMPLE_SYNC: Diff computed: 0 to add, 0 to update, 3 to delete
SIMPLE_SYNC: Files to DELETE: ["old1.txt", "old2.txt", "old3.txt"]
```

---

## Manual Verification

If you want to manually verify hashing is working:

### PowerShell command to hash a file:
```powershell
$bytes = [System.IO.File]::ReadAllBytes("C:\path\to\file.txt")
$hasher = [System.Security.Cryptography.SHA256]::Create()
$hash = $hasher.ComputeHash($bytes)
$hashString = [System.BitConverter]::ToString($hash).Replace("-", "").ToLower()
Write-Host $hashString
```

Compare this with the hash in the logs to verify correctness.

---

## What Was Changed

### 1. Path Normalization
```rust
// Before
path: relative_path.to_string_lossy().to_string(),

// After
let normalized_path = relative_path
    .to_string_lossy()
    .replace('\\', "/");
```

This ensures Windows paths like `src\main.rs` become `src/main.rs` for consistent comparison.

### 2. Enhanced Logging
Added detailed logs at every step:
- Manifest creation
- File hashing
- Diff computation
- Each file being added/updated/deleted

### 3. Error Context
Better error messages when things fail:
- "No project_root available"
- "No peer_id available"
- Parse errors with context

---

## Diagnostic Checklist

When debugging, check these in order:

- [ ] **Logs show "Creating file manifest"** → Host is trying to create manifest
- [ ] **Logs show "Created manifest with X files"** → Manifest created successfully (X > 0)
- [ ] **Logs show "Sent file manifest"** → Host sent to joiner
- [ ] **Logs show "Received file manifest"** → Joiner received it
- [ ] **Logs show "Parsed manifest with X files"** → Joiner can parse it
- [ ] **Logs show "Local manifest has Y files"** → Joiner's local manifest created
- [ ] **Logs show "File to ADD/UPDATE/DELETE"** → Differences found
- [ ] **Logs show "Changes detected, showing sync UI"** → UI should update
- [ ] **UI shows FileSync tab** → Tab switched
- [ ] **UI shows file counts** → Diff is displayed

If any step fails, that's where the problem is!

---

## Quick Fix Checklist

If diffs still aren't showing:

1. **Check project_root is set**:
   ```rust
   // In multiplayer window initialization
   tracing::info!("Project root: {:?}", self.project_root);
   ```

2. **Verify files exist**:
   ```bash
   # On host
   ls -R <project_root>  # Should show files
   ```

3. **Check ignore patterns**:
   - Make sure your files aren't all being ignored
   - Temporarily disable ignores to test

4. **Verify WebSocket connection**:
   - Check both peers stay connected
   - Look for disconnect/reconnect logs

5. **Check UI state**:
   ```rust
   tracing::info!("pending_file_sync: {:?}", self.pending_file_sync.is_some());
   tracing::info!("current_tab: {:?}", self.current_tab);
   ```

---

## Expected Log Flow (Successful Sync)

```
[HOST] CREATE_SESSION: Connected
[HOST] CREATE_SESSION: Received RequestFileManifest from <joiner>
[HOST] HOST: Creating file manifest for <path>
[HOST] SIMPLE_SYNC: Creating manifest for <path>
[HOST] SIMPLE_SYNC: Added file: src/main.rs (hash: abc12345, size: 1024)
[HOST] SIMPLE_SYNC: Added file: Cargo.toml (hash: def67890, size: 512)
[HOST] SIMPLE_SYNC: Created manifest with 2 files
[HOST] HOST: Created manifest with 2 files
[HOST] HOST: Serialized manifest to 350 bytes
[HOST] HOST: Sent file manifest

[JOINER] JOIN_SESSION: Connected
[JOINER] JOIN_SESSION: Sent RequestFileManifest
[JOINER] JOIN_SESSION: Received file manifest from <host> (350 bytes)
[JOINER] JOIN_SESSION: Project root is <path>
[JOINER] JOIN_SESSION: Parsed manifest with 2 files
[JOINER] SIMPLE_SYNC: Remote sample files: ["src/main.rs", "Cargo.toml"]
[JOINER] SIMPLE_SYNC: Computing diff against remote manifest (2 files)
[JOINER] SIMPLE_SYNC: Creating manifest for <path>
[JOINER] SIMPLE_SYNC: Created manifest with 0 files
[JOINER] SIMPLE_SYNC: Local manifest has 0 files
[JOINER] SIMPLE_SYNC: File to ADD: src/main.rs
[JOINER] SIMPLE_SYNC: File to ADD: Cargo.toml
[JOINER] SIMPLE_SYNC: Diff computed: 2 to add, 0 to update, 0 to delete
[JOINER] JOIN_SESSION: Computed diff - 2 to add, 0 to update, 0 to delete
[JOINER] JOIN_SESSION: Changes detected, showing sync UI
[JOINER] JOIN_SESSION: Set pending_file_sync and switched to FileSync tab
[JOINER] Rendering FileSync tab with pending diff

→ UI SHOWS SYNC DIALOG WITH 2 FILES TO ADD
```

---

## Need More Help?

Run with `RUST_LOG=debug` and send me the logs showing:
1. From "Creating file manifest" to "Sent file manifest" (host side)
2. From "Received file manifest" to "Changes detected" (joiner side)

This will show exactly where the issue is!
