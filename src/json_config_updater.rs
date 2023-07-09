use serde_json::Value as JsonValue;

// Prerequisites:
// - Versions are always registered in order from oldest to newest
// - There are no versions written in json file that don't have updater functions. in other words,
// if a version doesn't require updating to, never manually write that version to the json file
// - If no version if sound in the json, it is assumed that the json file has version before the
// first listed. If you want to have a version for that, you can create an empty patching function
// - Old update functions never change (instead, if bugs found, create a new version, or you need
// yourself validate all new possible paths of update that you create and find what they can break)
// - Version field name is never changes
// - If no update functions are provided, the json file is not changed

pub struct JsonConfigUpdater {
    latest_version: String,
    version_field_name: String,
    patchers: Vec<Patcher>,
}

#[derive(Debug, PartialEq)]
pub enum UpdateResult {
    Updated,
    NoUpdateNeeded,
    Error(String),
}

struct Patcher {
    version_to: String,
    function: fn(&mut JsonValue),
}

impl JsonConfigUpdater {
    pub fn new(version_field_name: &str) -> Self {
        Self {
            latest_version: String::new(),
            version_field_name: version_field_name.to_string(),
            patchers: Vec::new(),
        }
    }

    pub fn add_update_function(&mut self, version_to: &str, patcher_function: fn(&mut JsonValue)) {
        let version_string = version_to.to_string();
        self.patchers.push(Patcher {
            version_to: version_string.clone(),
            function: patcher_function,
        });
        self.latest_version = version_string;
    }

    pub fn update_json(&self, json: &mut JsonValue) -> UpdateResult {
        if self.patchers.is_empty() {
            return UpdateResult::NoUpdateNeeded;
        }

        let version = json[&self.version_field_name].as_str();

        let first_patcher_idx = if let Some(version_string) = version {
            let found_idx = self
                .patchers
                .iter()
                .rposition(|patcher| patcher.version_to == version_string);
            if let Some(found_idx) = found_idx {
                found_idx + 1
            } else {
                return UpdateResult::Error(format!("Version {} is not found during patching process. Make sure you didn't input the version manually into the json", version_string));
            }
        } else {
            0
        };

        if first_patcher_idx == self.patchers.len() {
            return UpdateResult::NoUpdateNeeded;
        }

        for patcher in &self.patchers[first_patcher_idx..] {
            (patcher.function)(json);
        }

        // bound check is done above
        json[&self.version_field_name] =
            serde_json::Value::String(self.patchers.last().unwrap().version_to.clone());

        return UpdateResult::Updated;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn patcher_function_1(json: &mut JsonValue) {
        json["a"] = json!(15);
    }

    fn patcher_function_2(json: &mut JsonValue) {
        json["b"] = json!("V");
    }

    fn patcher_function_3(json: &mut JsonValue) {
        json["c"] = json!("d");
    }

    #[test]
    fn test_patcher_without_versions_does_nothing() {
        let test_json = r#"{"a": 10, "b": "t"}"#;
        let mut json_value: JsonValue = serde_json::from_str(test_json).unwrap();

        let json_config_updater = JsonConfigUpdater::new("version");
        let result = json_config_updater.update_json(&mut json_value);

        assert_eq!(json_value, json!({"a": 10, "b": "t"}));
        assert_eq!(result, UpdateResult::NoUpdateNeeded);
    }

    #[test]
    fn test_patcher_without_previous_version_applies_all_patches() {
        let test_json = r#"{"a": 10, "b": "t"}"#;
        let mut json_value: JsonValue = serde_json::from_str(test_json).unwrap();

        let mut json_config_updater = JsonConfigUpdater::new("version");
        json_config_updater.add_update_function("1", patcher_function_1);
        json_config_updater.add_update_function("2", patcher_function_2);
        json_config_updater.add_update_function("3", patcher_function_3);
        let result = json_config_updater.update_json(&mut json_value);

        assert_eq!(
            json_value,
            json!({"a": 15, "b": "V", "c": "d", "version": "3"})
        );
        assert_eq!(result, UpdateResult::Updated);
    }

    #[test]
    fn test_patcher_with_an_old_version_applies_patches_from_the_next_version() {
        let test_json = r#"{"a": 10, "b": "t", "version": "1"}"#;
        let mut json_value: JsonValue = serde_json::from_str(test_json).unwrap();

        let mut json_config_updater = JsonConfigUpdater::new("version");
        json_config_updater.add_update_function("1", patcher_function_1);
        json_config_updater.add_update_function("2", patcher_function_2);
        json_config_updater.add_update_function("3", patcher_function_3);
        let result = json_config_updater.update_json(&mut json_value);

        assert_eq!(
            json_value,
            json!({"a": 10, "b": "V", "c": "d", "version": "3"})
        );
        assert_eq!(result, UpdateResult::Updated);
    }

    #[test]
    fn test_patcher_with_the_latest_version_does_nothing() {
        let test_json = r#"{"a": 10, "b": "t", "version": "3"}"#;
        let mut json_value: JsonValue = serde_json::from_str(test_json).unwrap();

        let mut json_config_updater = JsonConfigUpdater::new("version");
        json_config_updater.add_update_function("1", patcher_function_1);
        json_config_updater.add_update_function("2", patcher_function_2);
        json_config_updater.add_update_function("3", patcher_function_3);
        let result = json_config_updater.update_json(&mut json_value);

        assert_eq!(json_value, json!({"a": 10, "b": "t", "version": "3"}));
        assert_eq!(result, UpdateResult::NoUpdateNeeded);
    }

    #[test]
    fn test_patcher_with_invalid_version_does_nothing() {
        let test_json = r#"{"a": 10, "b": "t", "version": "4"}"#;
        let mut json_value: JsonValue = serde_json::from_str(test_json).unwrap();

        let mut json_config_updater = JsonConfigUpdater::new("version");
        json_config_updater.add_update_function("1", patcher_function_1);
        json_config_updater.add_update_function("2", patcher_function_2);
        json_config_updater.add_update_function("3", patcher_function_3);
        let result = json_config_updater.update_json(&mut json_value);

        assert_eq!(json_value, json!({"a": 10, "b": "t", "version": "4"}));
        assert_eq!(result, UpdateResult::Error("Version 4 is not found during patching process. Make sure you didn't input the version manually into the json".to_string()));
    }
}
