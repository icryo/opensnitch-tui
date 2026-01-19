//! Process information utilities

/// Get the basename of a path
pub fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Truncate a path to fit display, keeping the basename
pub fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }

    let base = basename(path);
    if base.len() >= max_len {
        return format!("...{}", &base[base.len().saturating_sub(max_len - 3)..]);
    }

    let remaining = max_len - base.len() - 4; // -4 for ".../
    if remaining > 0 && path.len() > base.len() + 1 {
        let prefix = &path[..remaining.min(path.len() - base.len() - 1)];
        format!("{}.../{}", prefix, base)
    } else {
        format!(".../{}", base)
    }
}

/// Format command line arguments
pub fn format_cmdline(path: &str, args: &[String]) -> String {
    if args.is_empty() {
        path.to_string()
    } else {
        format!("{} {}", path, args.join(" "))
    }
}

/// Get user name from UID (placeholder - would need system lookup)
pub fn uid_to_name(uid: u32) -> String {
    match uid {
        0 => "root".to_string(),
        1000 => "user".to_string(),
        _ => uid.to_string(),
    }
}
