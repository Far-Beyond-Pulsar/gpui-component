# Script to fix imports in all ui-crates files

$replacements = @(
    # Fix crate references
    @{ Pattern = 'use crate::ui::common::'; Replacement = 'use ui_common::' }
    @{ Pattern = 'use crate::ui::core::'; Replacement = 'use ui_core::' }
    @{ Pattern = 'use crate::ui::editors::'; Replacement = 'use ui_editor::editors::' }
    @{ Pattern = 'use crate::ui::helpers::'; Replacement = 'use ui_common::helpers::' }
    @{ Pattern = 'use crate::ui::windows::editor::'; Replacement = 'use ui_editor::' }
    @{ Pattern = 'use crate::ui::windows::entry_'; Replacement = 'use ui_entry::' }
    @{ Pattern = 'use crate::ui::windows::settings'; Replacement = 'use ui_settings::settings' }
    @{ Pattern = 'use crate::ui::windows::terminal'; Replacement = 'use ui_terminal::' }
    @{ Pattern = 'use crate::ui::windows::multiplayer_window'; Replacement = 'use ui_multiplayer' }
    @{ Pattern = 'use crate::ui::windows::problems_window'; Replacement = 'use ui_problems::window' }
    @{ Pattern = 'use crate::ui::windows::file_manager_window'; Replacement = 'use ui_file_manager::window' }
    @{ Pattern = 'use crate::ui::windows::loading_window'; Replacement = 'use ui_entry::loading_window' }
    
    # Fix re-exports
    @{ Pattern = 'pub use crate::ui::common::'; Replacement = 'pub use ui_common::' }
    @{ Pattern = 'pub use crate::ui::core::'; Replacement = 'pub use ui_core::' }
    
    # Fix remaining crate::ui references to point to engine crate
    @{ Pattern = 'use crate::ui::'; Replacement = 'use pulsar_engine::ui::' }
    @{ Pattern = 'pub use crate::ui::'; Replacement = 'pub use pulsar_engine::ui::' }
    
    # Fix crate::assets and other engine references
    @{ Pattern = 'use crate::assets'; Replacement = 'use pulsar_engine::assets' }
    @{ Pattern = 'use crate::Assets'; Replacement = 'use pulsar_engine::Assets' }
    @{ Pattern = 'use crate::render'; Replacement = 'use pulsar_engine::render' }
    @{ Pattern = 'use crate::settings'; Replacement = 'use pulsar_engine::settings' }
    @{ Pattern = 'use crate::themes'; Replacement = 'use pulsar_engine::themes' }
    @{ Pattern = 'use crate::graph'; Replacement = 'use pulsar_engine::graph' }
    @{ Pattern = 'use crate::compiler'; Replacement = 'use pulsar_engine::compiler' }
)

$uiCrates = Get-ChildItem "ui-crates" -Directory

foreach ($crate in $uiCrates) {
    Write-Host "`nProcessing $($crate.Name)..." -ForegroundColor Cyan
    
    $files = Get-ChildItem "$($crate.FullName)\src" -Recurse -Filter "*.rs"
    
    foreach ($file in $files) {
        $content = Get-Content $file.FullName -Raw
        $originalContent = $content
        
        foreach ($replacement in $replacements) {
            $content = $content -replace [regex]::Escape($replacement.Pattern), $replacement.Replacement
        }
        
        if ($content -ne $originalContent) {
            Set-Content $file.FullName -Value $content -NoNewline
            Write-Host "  Updated: $($file.Name)" -ForegroundColor Green
        }
    }
}

Write-Host "`n=== Import fixes complete ===" -ForegroundColor Green
