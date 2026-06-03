//! URL construction for runtime downloads with mirror support.

use super::RuntimeKind;

/// Resolve the mirror base URL for a given mirror name.
fn mirror_base(mirror: &str) -> &str {
    match mirror {
        "aliyun" | "npmmirror" => "https://npmmirror.com/mirrors",
        "huawei" => "https://repo.huaweicloud.com",
        "huaweicloud" => "https://repo.huaweicloud.com",
        _ => "direct", // direct means use official source
    }
}

/// Build the download URL for a runtime of a given version.
pub fn build_download_url(kind: RuntimeKind, version: &str, mirror: &str) -> String {
    let base = mirror_base(mirror);
    match kind {
        RuntimeKind::Node => build_node_url(version, base),
        RuntimeKind::Java => build_java_url(version, base),
        RuntimeKind::Maven => build_maven_url(version, base),
    }
}

/// Build the filename of the downloaded archive (for caching/naming).
pub fn archive_filename(kind: RuntimeKind, version: &str) -> String {
    match kind {
        RuntimeKind::Node => format!("node-v{version}-win-x64.zip"),
        RuntimeKind::Java => format!("jdk-{version}-windows-x64.zip"),
        RuntimeKind::Maven => format!("apache-maven-{version}-bin.zip"),
    }
}

/// Expected root directory inside the archive after extraction.
pub fn archive_root_dir(kind: RuntimeKind, version: &str) -> String {
    match kind {
        RuntimeKind::Node => format!("node-v{version}-win-x64"),
        RuntimeKind::Java => format!("jdk-{version}"),   // Adoptium extracts to just "jdk-{version}"
        RuntimeKind::Maven => format!("apache-maven-{version}"),
    }
}

// ---- Node.js ----
fn build_node_url(version: &str, base: &str) -> String {
    let file = format!("node-v{version}-win-x64.zip");
    if base == "direct" {
        format!("https://nodejs.org/dist/v{version}/{file}")
    } else {
        format!("{base}/node/v{version}/{file}")
    }
}

// ---- Java (Adoptium Temurin) ----
fn build_java_url(version: &str, base: &str) -> String {
    // Extract feature version (e.g., "21" from "21.0.3")
    let feature = version.split('.').next().unwrap_or(version);
    let file_suffix = format!("jdk_x64_windows_hotspot_{}.zip", version.replace('.', "_"));
    // Feature version "8" is special: "jdk8u" instead of "jdk21u"
    let jdk_prefix = match feature {
        "8" => format!("jdk{}u", feature),
        f => format!("jdk{}u", f),
    };

    if base == "direct" {
        format!(
            "https://github.com/adoptium/temurin{feature}-binaries/releases/download/jdk-{version}/{jdk_prefix}-{file_suffix}"
        )
    } else {
        // Mirrors may not have GitHub releases; fallback to direct for Java for now
        format!(
            "https://github.com/adoptium/temurin{feature}-binaries/releases/download/jdk-{version}/{jdk_prefix}-{file_suffix}"
        )
    }
}

// ---- Maven ----
fn build_maven_url(version: &str, base: &str) -> String {
    let file = format!("apache-maven-{version}-bin.zip");
    let path = format!("/apache/maven/maven-3/{version}/binaries/{file}");

    if base == "direct" {
        format!("https://dlcdn.apache.org{path}")
    } else {
        format!("{base}{path}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_direct_url() {
        let url = build_download_url(RuntimeKind::Node, "22.11.0", "direct");
        assert!(url.contains("nodejs.org"));
        assert!(url.contains("v22.11.0"));
        assert!(url.ends_with(".zip"));
    }

    #[test]
    fn test_node_aliyun_url() {
        let url = build_download_url(RuntimeKind::Node, "22.11.0", "aliyun");
        assert!(url.contains("npmmirror.com"));
        assert!(url.contains("v22.11.0"));
    }

    #[test]
    fn test_java_direct_url() {
        let url = build_download_url(RuntimeKind::Java, "21.0.3", "direct");
        assert!(url.contains("github.com/adoptium"));
        assert!(url.contains("jdk-21.0.3"));
    }

    #[test]
    fn test_maven_direct_url() {
        let url = build_download_url(RuntimeKind::Maven, "3.9.6", "direct");
        assert!(url.contains("dlcdn.apache.org"));
        assert!(url.contains("maven-3/3.9.6"));
    }

    #[test]
    fn test_maven_mirror_url() {
        let url = build_download_url(RuntimeKind::Maven, "3.9.6", "huawei");
        assert!(url.contains("repo.huaweicloud.com"));
    }

    #[test]
    fn test_archive_filename() {
        assert_eq!(archive_filename(RuntimeKind::Node, "22.11.0"), "node-v22.11.0-win-x64.zip");
        assert_eq!(archive_filename(RuntimeKind::Maven, "3.9.6"), "apache-maven-3.9.6-bin.zip");
    }
}
