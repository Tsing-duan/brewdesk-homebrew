use std::path::Path;

#[derive(Default)]
pub(super) struct ApplicationMetadata(Option<plist::Dictionary>);

impl ApplicationMetadata {
    pub(super) fn read(app: &Path) -> Self {
        Self(
            plist::Value::from_file(app.join("Contents/Info.plist"))
                .ok()
                .and_then(plist::Value::into_dictionary),
        )
    }

    pub(super) fn text(&self, key: &str) -> Option<String> {
        let value = match self.0.as_ref()?.get(key)? {
            plist::Value::String(value) => value.trim().to_string(),
            plist::Value::Integer(value) => value.to_string(),
            plist::Value::Real(value) => value.to_string(),
            plist::Value::Boolean(value) => value.to_string(),
            _ => return None,
        };
        (!value.is_empty() && value != "null").then_some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn reads_xml_and_binary_metadata_with_the_same_fallback_values() {
        let root = std::env::temp_dir().join(format!("brewdesk-metadata-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("Contents")).unwrap();
        let mut dictionary = plist::Dictionary::new();
        dictionary.insert("CFBundleName".into(), " Demo ".into());
        dictionary.insert("CFBundleVersion".into(), plist::Value::Integer(42.into()));
        dictionary.insert("CFBundleIdentifier".into(), "local.demo".into());
        dictionary.insert("CFBundleDisplayName".into(), "".into());
        dictionary.insert("CFBundleShortVersionString".into(), "null".into());
        let value = plist::Value::Dictionary(dictionary);
        for binary in [false, true] {
            let path = root.join("Contents/Info.plist");
            if binary {
                value.to_file_binary(&path).unwrap();
            } else {
                value.to_file_xml(&path).unwrap();
            }
            let metadata = ApplicationMetadata::read(&root);
            assert_eq!(metadata.text("CFBundleName").as_deref(), Some("Demo"));
            assert_eq!(metadata.text("CFBundleVersion").as_deref(), Some("42"));
            assert_eq!(
                metadata.text("CFBundleIdentifier").as_deref(),
                Some("local.demo")
            );
            assert_eq!(metadata.text("CFBundleDisplayName"), None);
            assert_eq!(metadata.text("CFBundleShortVersionString"), None);
        }
        fs::write(root.join("Contents/Info.plist"), b"broken").unwrap();
        assert_eq!(ApplicationMetadata::read(&root).text("CFBundleName"), None);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(ApplicationMetadata::read(&root).text("CFBundleName"), None);
    }
}
