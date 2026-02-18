use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct IpAddresses;

impl IpAddresses {
    fn is_ipv4(s: &str) -> bool {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return false;
        }
        for part in &parts {
            match part.parse::<u16>() {
                Ok(n) if n <= 255 => {}
                _ => return false,
            }
        }
        true
    }

    fn is_ipv6(s: &str) -> bool {
        // Simple IPv6 validation
        let lower = s.to_lowercase();

        // Must contain at least one colon
        if !lower.contains(':') {
            return false;
        }

        // Must only contain hex digits and colons
        if !lower.chars().all(|c| c.is_ascii_hexdigit() || c == ':') {
            return false;
        }

        // Check for :: (collapsed zeros)
        if lower.contains("::") {
            // Can have at most one ::
            if lower.matches("::").count() > 1 {
                return false;
            }
            // Must have valid groups
            let parts: Vec<&str> = lower.split("::").collect();
            if parts.len() != 2 {
                return false;
            }
            let left_groups = if parts[0].is_empty() { 0 } else { parts[0].split(':').count() };
            let right_groups = if parts[1].is_empty() { 0 } else { parts[1].split(':').count() };
            if left_groups + right_groups > 7 {
                return false;
            }
            // Validate each group
            for part in parts {
                for group in part.split(':') {
                    if group.is_empty() {
                        continue;
                    }
                    if group.len() > 4 {
                        return false;
                    }
                    if !group.chars().all(|c| c.is_ascii_hexdigit()) {
                        return false;
                    }
                }
            }
            return true;
        }

        // Full form: 8 groups of hex
        let groups: Vec<&str> = lower.split(':').collect();
        if groups.len() != 8 {
            return false;
        }
        for group in &groups {
            if group.len() > 4 || group.is_empty() {
                return false;
            }
            if !group.chars().all(|c| c.is_ascii_hexdigit()) {
                return false;
            }
        }
        true
    }
}

impl Cop for IpAddresses {
    fn name(&self) -> &'static str {
        "Style/IpAddresses"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let string_node = match node.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let content = string_node.unescaped();
        let content_str = match std::str::from_utf8(&content) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        if content_str.is_empty() {
            return Vec::new();
        }

        let allowed = config.get_string_array("AllowedAddresses");

        // Check if it's in allowed addresses
        if let Some(ref allowed_list) = allowed {
            for addr in allowed_list {
                if addr.eq_ignore_ascii_case(content_str) {
                    return Vec::new();
                }
            }
        }

        let is_ip = Self::is_ipv4(content_str) || Self::is_ipv6(content_str);

        // Don't flag if the string contains other text (not just the IP)
        if is_ip {
            let loc = string_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Do not hardcode IP addresses.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IpAddresses, "cops/style/ip_addresses");
}
