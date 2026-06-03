//! URL construction for runtime downloads with mirror support.
//!
//! Different runtimes use different mirror infrastructures:
//! - Node: npmmirror.com mirrors Node.js binaries
//! - Maven: aliyun/huawei mirror Apache archives
//! - Java: GitHub releases (no standard mirror)

use super::RuntimeKind;

/// Build the download URL for a runtime of a given version and mirror.
pub fn build_download_url(kind: RuntimeKind, version: &str, mirror: &str) -> String {
    match kind {
        RuntimeKind::Node => build_node_url(version, mirror),
        RuntimeKind::Java => build_java_url(version, mirror),
        RuntimeKind::Maven => build_maven_url(version, mirror),
    }
}

/// Build the filename of the downloaded archive.
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
        RuntimeKind::Java => format!("jdk-{version}"),
        RuntimeKind::Maven => format!("apache-maven-{version}"),
    }
}

// ---- Node.js ----
// Official: https://nodejs.org/dist/v{version}/node-v{version}-win-x64.zip
// NPMMirror: https://npmmirror.com/mirrors/node/v{version}/node-v{version}-win-x64.zip
// Aliyun:   uses npmmirror as backend for Node
// Huawei:   https://repo.huaweicloud.com/nodejs/v{version}/node-v{version}-win-x64.zip
fn build_node_url(version: &str, mirror: &str) -> String {
    let file = format!("node-v{version}-win-x64.zip");
    let path = format!("/node/v{version}/{file}");
    match mirror {
        "aliyun" | "npmmirror" => format!("https://npmmirror.com/mirrors{path}"),
        "huawei" | "huaweicloud" => format!("https://repo.huaweicloud.com{path}"),
        _ => format!("https://nodejs.org/dist/v{version}/{file}"),
    }
}

// ---- Java (Adoptium Temurin) ----
// Only available from GitHub releases (no standard mirror).
// URL: https://github.com/adoptium/temurin{feature}-binaries/releases/download/jdk-{version}/OpenJDK{feature}U-jdk_x64_windows_hotspot_{dots_to_underscores}.zip
fn build_java_url(version: &str, _mirror: &str) -> String {
    let feature = version.split('.').next().unwrap_or(version);
    let file_suffix = format!("jdk_x64_windows_hotspot_{}.zip", version.replace('.', "_"));
    let jdk_prefix = match feature {
        "8" => format!("jdk{}u", feature),
        f => format!("jdk{}u", f),
    };

    format!(
        "https://github.com/adoptium/temurin{feature}-binaries/releases/download/jdk-{version}/{prefix}-{file_suffix}",
        feature = feature,
        version = version,
        prefix = jdk_prefix,
        file_suffix = file_suffix
    )
}

// ---- Maven ----
// Official: https://dlcdn.apache.org/maven/maven-3/{version}/binaries/apache-maven-{version}-bin.zip
// Aliyun:   https://mirrors.aliyun.com/apache/maven/maven-3/{version}/binaries/apache-maven-{version}-bin.zip
// Huawei:   https://repo.huaweicloud.com/apache/maven/maven-3/{version}/binaries/apache-maven-{version}-bin.zip
fn build_maven_url(version: &str, mirror: &str) -> String {
    let file = format!("apache-maven-{version}-bin.zip");
    let path = format!("/apache/maven/maven-3/{version}/binaries/{file}");
    match mirror {
        "aliyun" => format!("https://mirrors.aliyun.com{path}"),
        "huawei" | "huaweicloud" => format!("https://repo.huaweicloud.com{path}"),
        _ => format!("https://dlcdn.apache.org{path}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_direct_url() {
        let url = build_download_url(RuntimeKind::Node, "22.11.0", "direct");
        assert!(url.contains("nodejs.org"), "got: {url}");
        assert!(url.contains("v22.11.0"), "got: {url}");
    }

    #[test]
    fn test_node_aliyun_url() {
        let url = build_download_url(RuntimeKind::Node, "22.11.0", "aliyun");
        assert!(url.contains("npmmirror.com"), "got: {url}");
    }

    #[test]
    fn test_java_direct_url() {
        let url = build_download_url(RuntimeKind::Java, "21.0.3", "direct");
        assert!(url.contains("adoptium"), "got: {url}");
        assert!(url.contains("jdk-21.0.3"), "got: {url}");
    }

    #[test]
    fn test_maven_direct_url() {
        let url = build_download_url(RuntimeKind::Maven, "3.9.9", "direct");
        assert!(url.contains("dlcdn.apache.org"), "got: {url}");
        assert!(url.contains("maven-3/3.9.9"), "got: {url}");
    }

    #[test]
    fn test_maven_aliyun_url() {
        let url = build_download_url(RuntimeKind::Maven, "3.9.9", "aliyun");
        assert!(url.contains("mirrors.aliyun.com"), "got: {url}");
    }

    #[test]
    fn test_maven_huawei_url() {
        let url = build_download_url(RuntimeKind::Maven, "3.9.9", "huawei");
        assert!(url.contains("repo.huaweicloud.com"), "got: {url}");
    }

    #[test]
    fn test_archive_filename() {
        assert_eq!(
            archive_filename(RuntimeKind::Node, "22.11.0"),
            "node-v22.11.0-win-x64.zip"
        );
        assert_eq!(
            archive_filename(RuntimeKind::Maven, "3.9.9"),
            "apache-maven-3.9.9-bin.zip"
        );
    }
}
