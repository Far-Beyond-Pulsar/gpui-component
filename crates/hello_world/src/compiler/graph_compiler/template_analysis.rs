//! # Template Analysis
//!
//! Analyzes node templates to determine their type and execution characteristics.
//!
//! This module inspects template structure to classify them as:
//! - Pure expressions (no function wrapper)
//! - Simple functions (function with no exec placeholders)
//! - Control flow (function with exec placeholders)
//!
//! ## Template Analysis Algorithm
//!
//! 1. Check if template has a function wrapper (`fn`)
//! 2. If no function, it's a pure expression
//! 3. If function exists, scan for execution placeholders (`@[pulsar_exec_*]@`)
//! 4. No placeholders = simple function, has placeholders = control flow

use std::collections::HashMap;
use tron::TronTemplate;
use super::template_type::TemplateType;

impl super::GraphCompiler {
    /// Pre-analyze all templates to determine their type.
    ///
    /// This caches the template type for faster compilation. A template is:
    /// - **PureExpression** if it has no function wrapper
    /// - **SimpleFunction** if it has a function but no exec placeholders
    /// - **ControlFlow** if it has exec placeholders
    pub(super) fn analyze_all_templates(&mut self) {
        println!("[COMPILER] Analyzing {} templates...", self.templates.len());

        for (node_type, template) in &self.templates {
            let template_type = self.analyze_template_type(template);
            println!("[COMPILER]   {}: {:?}", node_type, template_type);
            self.template_types.insert(node_type.clone(), template_type);
        }
    }

    /// Analyze a single template to determine its type.
    ///
    /// ## Algorithm
    ///
    /// 1. Check for function wrapper
    /// 2. Extract all execution placeholders
    /// 3. Classify based on structure
    pub(super) fn analyze_template_type(&self, template: &TronTemplate) -> TemplateType {
        // Get template content - we need to inspect it
        let template_str = format!("{:?}", template);

        // Check if it has a function wrapper
        if !template_str.contains("fn ") {
            return TemplateType::PureExpression;
        }

        // Find all execution placeholders
        let mut exec_placeholders = Vec::new();
        for part in template_str.split("@[") {
            if let Some(end) = part.find("]@") {
                let placeholder = &part[..end];
                if placeholder.starts_with("pulsar_exec_") {
                    exec_placeholders.push(placeholder.to_string());
                }
            }
        }

        if exec_placeholders.is_empty() {
            TemplateType::SimpleFunction
        } else {
            TemplateType::ControlFlow { exec_placeholders }
        }
    }

    /// Get the cached template type for a node type.
    ///
    /// Returns an error if the template type hasn't been analyzed yet.
    pub(super) fn get_template_type(&self, node_type: &str) -> Result<&TemplateType, String> {
        self.template_types
            .get(node_type)
            .ok_or_else(|| format!("Template type not found for: {}", node_type))
    }
}