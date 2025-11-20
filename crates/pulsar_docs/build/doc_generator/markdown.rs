/// Markdown generation module
/// 
/// Converts extracted documentation structures into well-formatted Markdown.
/// Generates comprehensive, navigable documentation with:
/// - Table of contents
/// - Syntax highlighting for code
/// - Cross-references
/// - Examples
/// - Source links

use std::error::Error;
use std::fmt::Write;
use super::types::*;
use super::workspace::CrateInfo;

/// Generate markdown documentation for a crate
/// 
/// # Arguments
/// * `docs` - The extracted documentation
/// * `crate_info` - Information about the crate
/// 
/// # Returns
/// Complete markdown string
pub fn generate_markdown(
    docs: &CrateDocumentation,
    crate_info: &CrateInfo,
) -> Result<String, Box<dyn Error>> {
    let mut md = String::new();
    
    // Title and metadata
    writeln!(md, "# {}", docs.name)?;
    writeln!(md)?;
    writeln!(md, "**Version:** {}", docs.version)?;
    writeln!(md)?;
    
    if let Some(desc) = &docs.description {
        writeln!(md, "{}", desc)?;
        writeln!(md)?;
    }
    
    // Table of contents
    writeln!(md, "## Table of Contents")?;
    writeln!(md)?;
    
    if !docs.modules.is_empty() {
        writeln!(md, "- [Modules](#modules)")?;
    }
    if !docs.structs.is_empty() {
        writeln!(md, "- [Structs](#structs)")?;
    }
    if !docs.enums.is_empty() {
        writeln!(md, "- [Enums](#enums)")?;
    }
    if !docs.traits.is_empty() {
        writeln!(md, "- [Traits](#traits)")?;
    }
    if !docs.functions.is_empty() {
        writeln!(md, "- [Functions](#functions)")?;
    }
    if !docs.macros.is_empty() {
        writeln!(md, "- [Macros](#macros)")?;
    }
    if !docs.constants.is_empty() {
        writeln!(md, "- [Constants](#constants)")?;
    }
    if !docs.type_aliases.is_empty() {
        writeln!(md, "- [Type Aliases](#type-aliases)")?;
    }
    writeln!(md)?;
    
    // Generate sections
    generate_structs_section(&mut md, &docs.structs)?;
    generate_enums_section(&mut md, &docs.enums)?;
    generate_traits_section(&mut md, &docs.traits)?;
    generate_functions_section(&mut md, &docs.functions)?;
    generate_macros_section(&mut md, &docs.macros)?;
    generate_constants_section(&mut md, &docs.constants)?;
    generate_type_aliases_section(&mut md, &docs.type_aliases)?;
    
    Ok(md)
}

fn generate_structs_section(md: &mut String, structs: &[StructDoc]) -> Result<(), Box<dyn Error>> {
    if structs.is_empty() {
        return Ok(());
    }
    
    writeln!(md, "## Structs")?;
    writeln!(md)?;
    
    for s in structs {
        // Only document public items
        if s.visibility != Visibility::Public {
            continue;
        }
        
        writeln!(md, "### `{}`", s.name)?;
        writeln!(md)?;
        
        if let Some(doc) = &s.doc_comment {
            writeln!(md, "{}", doc)?;
            writeln!(md)?;
        }
        
        // Generics
        if !s.generics.is_empty() {
            write!(md, "**Generic Parameters:** ")?;
            for (i, g) in s.generics.iter().enumerate() {
                if i > 0 {
                    write!(md, ", ")?;
                }
                write!(md, "`{}`", g.name)?;
                if !g.bounds.is_empty() {
                    write!(md, ": {}", g.bounds.join(" + "))?;
                }
            }
            writeln!(md)?;
            writeln!(md)?;
        }
        
        // Fields
        if !s.fields.is_empty() {
            writeln!(md, "**Fields:**")?;
            writeln!(md)?;
            for field in &s.fields {
                write!(md, "- `{}`: `{}`", field.name, field.type_)?;
                if let Some(doc) = &field.doc_comment {
                    write!(md, " - {}", doc.lines().next().unwrap_or(""))?;
                }
                writeln!(md)?;
            }
            writeln!(md)?;
        }
        
        // Source code
        writeln!(md, "<details>")?;
        writeln!(md, "<summary>Source Code</summary>")?;
        writeln!(md)?;
        writeln!(md, "```rust")?;
        writeln!(md, "{}", s.source_code)?;
        writeln!(md, "```")?;
        writeln!(md, "</details>")?;
        writeln!(md)?;
        
        writeln!(md, "---")?;
        writeln!(md)?;
    }
    
    Ok(())
}

fn generate_enums_section(md: &mut String, enums: &[EnumDoc]) -> Result<(), Box<dyn Error>> {
    if enums.is_empty() {
        return Ok(());
    }
    
    writeln!(md, "## Enums")?;
    writeln!(md)?;
    
    for e in enums {
        if e.visibility != Visibility::Public {
            continue;
        }
        
        writeln!(md, "### `{}`", e.name)?;
        writeln!(md)?;
        
        if let Some(doc) = &e.doc_comment {
            writeln!(md, "{}", doc)?;
            writeln!(md)?;
        }
        
        // Variants
        if !e.variants.is_empty() {
            writeln!(md, "**Variants:**")?;
            writeln!(md)?;
            for variant in &e.variants {
                write!(md, "- `{}`", variant.name)?;
                
                match &variant.fields {
                    VariantFields::Unit => {}
                    VariantFields::Tuple(types) => {
                        write!(md, "({})", types.join(", "))?;
                    }
                    VariantFields::Struct(fields) => {
                        writeln!(md, " {{")?;
                        for field in fields {
                            writeln!(md, "    {}: {},", field.name, field.type_)?;
                        }
                        write!(md, "  }}")?;
                    }
                }
                
                if let Some(doc) = &variant.doc_comment {
                    write!(md, " - {}", doc.lines().next().unwrap_or(""))?;
                }
                writeln!(md)?;
            }
            writeln!(md)?;
        }
        
        // Source code
        writeln!(md, "<details>")?;
        writeln!(md, "<summary>Source Code</summary>")?;
        writeln!(md)?;
        writeln!(md, "```rust")?;
        writeln!(md, "{}", e.source_code)?;
        writeln!(md, "```")?;
        writeln!(md, "</details>")?;
        writeln!(md)?;
        
        writeln!(md, "---")?;
        writeln!(md)?;
    }
    
    Ok(())
}

fn generate_traits_section(md: &mut String, traits: &[TraitDoc]) -> Result<(), Box<dyn Error>> {
    if traits.is_empty() {
        return Ok(());
    }
    
    writeln!(md, "## Traits")?;
    writeln!(md)?;
    
    for t in traits {
        if t.visibility != Visibility::Public {
            continue;
        }
        
        writeln!(md, "### `{}`", t.name)?;
        writeln!(md)?;
        
        if let Some(doc) = &t.doc_comment {
            writeln!(md, "{}", doc)?;
            writeln!(md)?;
        }
        
        // Methods
        if !t.methods.is_empty() {
            writeln!(md, "**Methods:**")?;
            writeln!(md)?;
            for method in &t.methods {
                write!(md, "- `{}`", method.name)?;
                
                // Parameters
                write!(md, "(")?;
                if method.self_param.is_some() {
                    write!(md, "&self")?;
                    if !method.parameters.is_empty() {
                        write!(md, ", ")?;
                    }
                }
                for (i, param) in method.parameters.iter().enumerate() {
                    if i > 0 {
                        write!(md, ", ")?;
                    }
                    write!(md, "{}: {}", param.name, param.type_)?;
                }
                write!(md, ")")?;
                
                // Return type
                if let Some(ret) = &method.return_type {
                    write!(md, " -> {}", ret)?;
                }
                
                if let Some(doc) = &method.doc_comment {
                    write!(md, " - {}", doc.lines().next().unwrap_or(""))?;
                }
                writeln!(md)?;
            }
            writeln!(md)?;
        }
        
        writeln!(md, "---")?;
        writeln!(md)?;
    }
    
    Ok(())
}

fn generate_functions_section(md: &mut String, functions: &[FunctionDoc]) -> Result<(), Box<dyn Error>> {
    if functions.is_empty() {
        return Ok(());
    }
    
    writeln!(md, "## Functions")?;
    writeln!(md)?;
    
    for f in functions {
        if f.visibility != Visibility::Public {
            continue;
        }
        
        // Function signature
        write!(md, "### `")?;
        if f.is_const {
            write!(md, "const ")?;
        }
        if f.is_async {
            write!(md, "async ")?;
        }
        if f.is_unsafe {
            write!(md, "unsafe ")?;
        }
        write!(md, "fn {}", f.name)?;
        
        // Generics
        if !f.generics.is_empty() {
            write!(md, "<")?;
            for (i, g) in f.generics.iter().enumerate() {
                if i > 0 {
                    write!(md, ", ")?;
                }
                write!(md, "{}", g.name)?;
            }
            write!(md, ">")?;
        }
        
        // Parameters
        write!(md, "(")?;
        for (i, param) in f.parameters.iter().enumerate() {
            if i > 0 {
                write!(md, ", ")?;
            }
            write!(md, "{}: {}", param.name, param.type_)?;
        }
        write!(md, ")")?;
        
        // Return type
        if let Some(ret) = &f.return_type {
            write!(md, " -> {}", ret)?;
        }
        
        writeln!(md, "`")?;
        writeln!(md)?;
        
        if let Some(doc) = &f.doc_comment {
            writeln!(md, "{}", doc)?;
            writeln!(md)?;
        }
        
        // Source code
        writeln!(md, "<details>")?;
        writeln!(md, "<summary>Source Code</summary>")?;
        writeln!(md)?;
        writeln!(md, "```rust")?;
        writeln!(md, "{}", f.source_code)?;
        writeln!(md, "```")?;
        writeln!(md, "</details>")?;
        writeln!(md)?;
        
        writeln!(md, "---")?;
        writeln!(md)?;
    }
    
    Ok(())
}

fn generate_macros_section(md: &mut String, macros: &[MacroDoc]) -> Result<(), Box<dyn Error>> {
    if macros.is_empty() {
        return Ok(());
    }
    
    writeln!(md, "## Macros")?;
    writeln!(md)?;
    
    for m in macros {
        writeln!(md, "### `{}!`", m.name)?;
        writeln!(md)?;
        
        if let Some(doc) = &m.doc_comment {
            writeln!(md, "{}", doc)?;
            writeln!(md)?;
        }
        
        writeln!(md, "```rust")?;
        writeln!(md, "{}", m.source_code)?;
        writeln!(md, "```")?;
        writeln!(md)?;
        
        writeln!(md, "---")?;
        writeln!(md)?;
    }
    
    Ok(())
}

fn generate_constants_section(md: &mut String, constants: &[ConstantDoc]) -> Result<(), Box<dyn Error>> {
    if constants.is_empty() {
        return Ok(());
    }
    
    writeln!(md, "## Constants")?;
    writeln!(md)?;
    
    for c in constants {
        if c.visibility != Visibility::Public {
            continue;
        }
        
        write!(md, "### `const {}: {}", c.name, c.type_)?;
        if let Some(value) = &c.value {
            write!(md, " = {}", value)?;
        }
        writeln!(md, "`")?;
        writeln!(md)?;
        
        if let Some(doc) = &c.doc_comment {
            writeln!(md, "{}", doc)?;
            writeln!(md)?;
        }
        
        writeln!(md, "---")?;
        writeln!(md)?;
    }
    
    Ok(())
}

fn generate_type_aliases_section(md: &mut String, type_aliases: &[TypeAliasDoc]) -> Result<(), Box<dyn Error>> {
    if type_aliases.is_empty() {
        return Ok(());
    }
    
    writeln!(md, "## Type Aliases")?;
    writeln!(md)?;
    
    for t in type_aliases {
        if t.visibility != Visibility::Public {
            continue;
        }
        
        writeln!(md, "### `type {} = {}`", t.name, t.target_type)?;
        writeln!(md)?;
        
        if let Some(doc) = &t.doc_comment {
            writeln!(md, "{}", doc)?;
            writeln!(md)?;
        }
        
        writeln!(md, "---")?;
        writeln!(md)?;
    }
    
    Ok(())
}
