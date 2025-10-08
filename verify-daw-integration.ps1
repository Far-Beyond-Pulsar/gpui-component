#!/usr/bin/env pwsh
# DAW Integration Verification Script

Write-Host "🎵 DAW Editor Integration Verification" -ForegroundColor Cyan
Write-Host "=" * 50

$baseDir = "C:\Users\redst\OneDrive\Documents\GitHub\Zenyx\Pulsar-Native"
$allPassed = $true

function Test-Integration {
    param([string]$name, [scriptblock]$test)
    
    Write-Host -NoNewline "Testing: $name... "
    try {
        $result = & $test
        if ($result) {
            Write-Host "✅ PASS" -ForegroundColor Green
            return $true
        } else {
            Write-Host "❌ FAIL" -ForegroundColor Red
            return $false
        }
    } catch {
        Write-Host "❌ ERROR: $_" -ForegroundColor Red
        return $false
    }
}

Write-Host "`n📦 File Structure Checks" -ForegroundColor Yellow
$allPassed = (Test-Integration "DAW module exists" {
    Test-Path "$baseDir\crates\engine\src\ui\panels\daw_editor\mod.rs"
}) -and $allPassed

$allPassed = (Test-Integration "Audio types module exists" {
    Test-Path "$baseDir\crates\engine\src\ui\panels\daw_editor\audio_types.rs"
}) -and $allPassed

$allPassed = (Test-Integration "UI module exists" {
    Test-Path "$baseDir\crates\engine\src\ui\panels\daw_editor\ui.rs"
}) -and $allPassed

$allPassed = (Test-Integration "GPU shaders exist" {
    (Test-Path "$baseDir\crates\engine\src\ui\panels\daw_editor\shaders\convolution.wgsl") -and
    (Test-Path "$baseDir\crates\engine\src\ui\panels\daw_editor\shaders\fft_eq.wgsl")
}) -and $allPassed

$allPassed = (Test-Integration "Demo project exists" {
    Test-Path "$baseDir\test_game\src\assets\audio\demo_music.pdaw"
}) -and $allPassed

Write-Host "`n🔌 Engine Integration Checks" -ForegroundColor Yellow
$allPassed = (Test-Integration "DAW exported from panels" {
    $content = Get-Content "$baseDir\crates\engine\src\ui\panels\mod.rs" -Raw
    $content -match "pub mod daw_editor" -and $content -match "pub use daw_editor::DawEditorPanel"
}) -and $allPassed

$allPassed = (Test-Integration "DawEditorPanel imported in app" {
    $content = Get-Content "$baseDir\crates\engine\src\ui\app.rs" -Raw
    $content -match "DawEditorPanel"
}) -and $allPassed

$allPassed = (Test-Integration "open_daw_tab method exists" {
    $content = Get-Content "$baseDir\crates\engine\src\ui\app.rs" -Raw
    $content -match "fn open_daw_tab"
}) -and $allPassed

$allPassed = (Test-Integration "daw_editors field exists" {
    $content = Get-Content "$baseDir\crates\engine\src\ui\app.rs" -Raw
    $content -match "daw_editors: Vec<Entity<DawEditorPanel>>"
}) -and $allPassed

$allPassed = (Test-Integration "FileType::DawProject exists" {
    $content = Get-Content "$baseDir\crates\engine\src\ui\file_manager_drawer.rs" -Raw
    $content -match "DawProject"
}) -and $allPassed

$allPassed = (Test-Integration "DAW file handler registered" {
    $content = Get-Content "$baseDir\crates\engine\src\ui\file_manager_drawer.rs" -Raw
    $content -match 'Some\("pdaw"\) => FileType::DawProject'
}) -and $allPassed

$allPassed = (Test-Integration "DAW click handler registered" {
    $content = Get-Content "$baseDir\crates\engine\src\ui\file_manager_drawer.rs" -Raw
    $content -match "FileType::Class \| FileType::Script \| FileType::DawProject"
}) -and $allPassed

Write-Host "`n🏗️ Build Checks" -ForegroundColor Yellow
$allPassed = (Test-Integration "Engine builds successfully" {
    Push-Location $baseDir
    $output = cargo check --package pulsar_engine 2>&1 | Out-String
    Pop-Location
    $output -match "Finished"
}) -and $allPassed

$allPassed = (Test-Integration "Dependencies added to Cargo.toml" {
    $content = Get-Content "$baseDir\crates\engine\Cargo.toml" -Raw
    $content -match "cpal" -and $content -match "wgpu" -and $content -match "hound"
}) -and $allPassed

Write-Host "`n📄 Demo Project Checks" -ForegroundColor Yellow
$allPassed = (Test-Integration "Demo project is valid JSON" {
    $json = Get-Content "$baseDir\test_game\src\assets\audio\demo_music.pdaw" -Raw
    try {
        $obj = $json | ConvertFrom-Json
        $obj.version -eq 1 -and $obj.name -eq "Demo Music Project"
    } catch {
        $false
    }
}) -and $allPassed

$allPassed = (Test-Integration "Demo has 4 tracks" {
    $json = Get-Content "$baseDir\test_game\src\assets\audio\demo_music.pdaw" -Raw | ConvertFrom-Json
    $json.tracks.Count -eq 4
}) -and $allPassed

Write-Host "`n" + "=" * 50
if ($allPassed) {
    Write-Host "✅ ALL TESTS PASSED - DAW is fully integrated!" -ForegroundColor Green
    Write-Host "`nThe DAW editor is ready to use!" -ForegroundColor Cyan
    Write-Host "Just click a .pdaw file in the file manager!" -ForegroundColor Cyan
    exit 0
} else {
    Write-Host "❌ SOME TESTS FAILED - Check errors above" -ForegroundColor Red
    exit 1
}
