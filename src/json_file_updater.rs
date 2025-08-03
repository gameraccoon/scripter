// Copyright (C) Pavel Grebnev 2023-2025
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use serde_json::Value as JsonValue;

/// JsonFileUpdater is a helper struct that handles updating json files to maintain compatibility
/// with the newer versions of the application.
/// It can be used to update any json files needed for the app, as long as prerequisites below are
/// met.
///
/// The idea is that you create an update function each time you need to change the json file format.
/// The update function are executed in a chain from an old version to the latest version, meaning
/// that there's only one possible update path from any previous version to the latest version.
/// The rest of the code can assume that the json file is always in the latest format and doesn't
/// need to perform any additional checks, handle missing fields or old formats.
///
/// Usage:
///  - Create a new JsonFileUpdater with a version field name.
///  - Register all update functions with add_update_function.
///  - Call update_json with a mutable reference to the json file.
///  - Use the resulting json file in the rest of the app code (and save if needed).
///
/// Prerequisites:
///  - The root of the json file is an object.
///  - Update functions are always registered in the order from oldest to newest.
///  - Version field in json file can contain only versions that are registered in the updater.
///    Which means:
///     - If a version doesn't require updating to, never write that version to the json file
///       (or you can create an empty update function for that version if you need to).
///     - There may be versions of the app that don't have corresponding version of the json file
///       (in case of a new version of the app that doesn't require updating the json file format).
///     - There may be versions of the json file that don't have corresponding version of the app
///       (in case of multiple changes of json format between two versions of the app).
///  - If no version field if found in the json, it is assumed that the json file has the version
///    before the first registered update function.
///  - You can create an empty update function if you want to have that version in json, but don't
///    need to update anything for that version. This only useful if you use this field for something
///    else than the updater.
///  - Version field name never changes, or its change should be patched in the json object, before
///    it is being passed to the update_json function.
///  - Updater functions should only be added and not changed, the only exception described in
///    Q&A section below.
///  - Updater functions should be self-contained, and should not call any of the app code that can
///    potentially change in the future. Otherwise, this will invalidate the whole purpose of the
///    updater and the app code would need to be versioned as well. This is not the place for DRY.
///  - When fields are added or removed, the update functions should still be created even if the
///    update can be handled by serde itself. This ensures that users who try to use older versions
///    of the app can always be detected correctly, and that you don't have surprises in the future
///    when you would need to add a new field with the same name.
///
/// Limitations:
///  - If no update functions are provided, the json file is not changed
///
/// Q&A:
///  - Q: How long I should keep the old update functions?
///
///    A: Ideally forever to ensure that the app can update from any version, in practice it depends
///       on your app policy of supporting old versions. If you want to drop support for updating
///       from some very old versions, you can remove the corresponding update functions. You can
///       also inform users that they need to run the older versions of the app first if they update
///       from a very old version, if that's possible.
///
///  - Q: I've introduced a bug in one of the update functions, I want to change it, what should I do?
///
///    A: Changing update functions can lead to branching of the update path, which is a huge pain to
///       deal with generally. If your bug doesn't introduce a data loss, just create a new version
///       of the app with a new update function that fixes the format. Don't change the buggy update
///       function.
///
///       If your bug introduces a data loss, then you need to apply this process:
///      - If the app version with the bug was never and will never be shipped to users, QA, or
///        other developers, you can of course fix the bug in the update function, but leave
///        a comment for a future reference
///      - Otherwise what you would usually want to do is:
///        1. Comment out the code of the buggy update function in the state that was shipped.
///           This is needed for future reference of anyone who is going to deal with new bugs that
///           may be introduced by branching of the update path.
///           When there's one bug, there are usually more.
///        2. Instead of that function body, set an indication that the non-buggy version was run.
///           This can be as simple as setting a boolean flag in the root of the json file. But you
///           may introduce something more structured if you want to keep it for a longer time.
///        3. Create a new version that branches on the indication and either runs the proper update
///           with the bug fixed, or does data recovery in case the buggy version was already run.
///           Remove the indication/flag that you've set on the previous step, unless you decided to
///           keep it for a longer time.
///        4. Document what happened for the future reference. Don't rely on people going through
///           VCS history to learn about a past branch in the update path.
///
///       This way you can limit the damage and make sure to merge the update paths as soon as
///       possible.
///
///       Now your update history would look like this, where B is the buggy version, B* is the
///       version that sets the flag, and C is the version that brings the users taking both paths
///       to the same state.
///
///
///       A
///       | \
///       B  B*
///       | /
///       C
///
pub struct JsonFileUpdater {
    latest_version: String,
    version_field_name: String,
    patchers: Vec<Patcher>,
}

#[derive(Debug, PartialEq)]
pub enum JsonFileUpdaterError {
    UnknownVersion {
        version: String,
        latest_version: String,
    },
}

#[derive(Debug, PartialEq)]
pub enum UpdateResult {
    Updated,
    NoUpdateNeeded,
    Error(JsonFileUpdaterError),
}

struct Patcher {
    version_to: String,
    function: fn(&mut JsonValue),
}

impl JsonFileUpdater {
    pub fn new(version_field_name: &str) -> Self {
        Self {
            latest_version: String::new(),
            version_field_name: version_field_name.to_string(),
            patchers: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_update_function(&mut self, version_to: &str, patcher_function: fn(&mut JsonValue)) {
        let version_string = version_to.to_string();
        self.patchers.push(Patcher {
            version_to: version_string.clone(),
            function: patcher_function,
        });
        self.latest_version = version_string;
    }

    #[allow(dead_code)]
    pub fn add_empty_update_function(&mut self, version_to: &str) {
        self.add_update_function(version_to, |_| {});
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
                return UpdateResult::Error(JsonFileUpdaterError::UnknownVersion {
                    version: version_string.to_string(),
                    latest_version: self.latest_version.clone(),
                });
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

        UpdateResult::Updated
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

        let json_file_updater = JsonFileUpdater::new("version");
        let result = json_file_updater.update_json(&mut json_value);

        assert_eq!(json_value, json!({"a": 10, "b": "t"}));
        assert_eq!(result, UpdateResult::NoUpdateNeeded);
    }

    #[test]
    fn test_patcher_without_previous_version_applies_all_patches() {
        let test_json = r#"{"a": 10, "b": "t"}"#;
        let mut json_value: JsonValue = serde_json::from_str(test_json).unwrap();

        let mut json_file_updater = JsonFileUpdater::new("version");
        json_file_updater.add_update_function("1", patcher_function_1);
        json_file_updater.add_update_function("2", patcher_function_2);
        json_file_updater.add_update_function("3", patcher_function_3);
        let result = json_file_updater.update_json(&mut json_value);

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

        let mut json_file_updater = JsonFileUpdater::new("version");
        json_file_updater.add_update_function("1", patcher_function_1);
        json_file_updater.add_update_function("2", patcher_function_2);
        json_file_updater.add_update_function("3", patcher_function_3);
        let result = json_file_updater.update_json(&mut json_value);

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

        let mut json_file_updater = JsonFileUpdater::new("version");
        json_file_updater.add_update_function("1", patcher_function_1);
        json_file_updater.add_update_function("2", patcher_function_2);
        json_file_updater.add_update_function("3", patcher_function_3);
        let result = json_file_updater.update_json(&mut json_value);

        assert_eq!(json_value, json!({"a": 10, "b": "t", "version": "3"}));
        assert_eq!(result, UpdateResult::NoUpdateNeeded);
    }

    #[test]
    fn test_patcher_with_invalid_version_does_nothing() {
        let test_json = r#"{"a": 10, "b": "t", "version": "4"}"#;
        let mut json_value: JsonValue = serde_json::from_str(test_json).unwrap();

        let mut json_file_updater = JsonFileUpdater::new("version");
        json_file_updater.add_update_function("1", patcher_function_1);
        json_file_updater.add_update_function("2", patcher_function_2);
        json_file_updater.add_update_function("3", patcher_function_3);
        let result = json_file_updater.update_json(&mut json_value);

        assert_eq!(json_value, json!({"a": 10, "b": "t", "version": "4"}));
        assert_eq!(
            result,
            UpdateResult::Error(JsonFileUpdaterError::UnknownVersion {
                version: "4".to_string(),
                latest_version: "3".to_string()
            })
        );
    }
}
