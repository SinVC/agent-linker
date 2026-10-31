use super::AddLinkableItem;
use crate::core::linkable::LinkableItemType;
use crate::core::symlink::LinkKind;
use crate::core::test_support::TestDir;
use crate::core::{
    paths::{DbPathReason, DbPathResolution},
    store::Store,
};
#[cfg(unix)]
use std::{ffi::OsString, os::unix::ffi::OsStringExt};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn resolution(temp_dir: &Path) -> DbPathResolution {
    DbPathResolution {
        path: temp_dir.join("agent-linker.db"),
        reason: DbPathReason::ExplicitDataDirEnv,
    }
}

#[test]
fn skill_registration_validates_source_and_calculates_default_link_path() {
    let temp_dir = TestDir::new("skill-add");
    let skill_dir = temp_dir.path().join("skill-source");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "Use this skill.").unwrap();
    let resolution = resolution(temp_dir.path());
    Store::migrate(&resolution).unwrap();

    let item = Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "writer".to_string(),
            alias: Some("writing-helper".to_string()),
            source_path: skill_dir.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();

    assert_eq!(item.source_kind, LinkKind::Directory);
    assert!(item.source_path.is_absolute());
    assert_eq!(item.link_name(), "writing-helper");
    assert_eq!(
        item.default_project_link_path().unwrap(),
        PathBuf::from(".agents")
            .join("skills")
            .join("writing-helper")
    );
}

#[test]
fn skill_registration_rejects_missing_or_empty_skill_md() {
    let temp_dir = TestDir::new("skill-invalid");
    let skill_dir = temp_dir.path().join("skill-source");
    fs::create_dir(&skill_dir).unwrap();
    let resolution = resolution(temp_dir.path());
    Store::migrate(&resolution).unwrap();

    let missing = Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "bad".to_string(),
            alias: None,
            source_path: skill_dir.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap_err();
    assert!(missing.to_string().contains("SKILL.md"));

    fs::write(skill_dir.join("SKILL.md"), "  \n").unwrap();
    let empty = Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "empty".to_string(),
            alias: None,
            source_path: skill_dir,
            default_target_dir: None,
            description: None,
        })
        .unwrap_err();
    assert!(empty.to_string().contains("must not be empty"));
}

#[test]
fn resource_registration_detects_file_and_directory_sources() {
    let temp_dir = TestDir::new("resource-kind");
    let file_source = temp_dir.path().join("notes.md");
    fs::write(&file_source, "notes").unwrap();
    let dir_source = temp_dir.path().join("assets");
    fs::create_dir(&dir_source).unwrap();
    let resolution = resolution(temp_dir.path());
    Store::migrate(&resolution).unwrap();

    let file = Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Resource,
            name: "notes".to_string(),
            alias: None,
            source_path: file_source,
            default_target_dir: Some(PathBuf::from(".agents").join("resources")),
            description: None,
        })
        .unwrap();
    assert_eq!(file.source_kind, LinkKind::File);
    assert_eq!(
        file.default_project_link_path().unwrap(),
        PathBuf::from(".agents").join("resources").join("notes.md")
    );

    let dir = Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Resource,
            name: "assets".to_string(),
            alias: Some("shared-assets".to_string()),
            source_path: dir_source,
            default_target_dir: Some(PathBuf::from("vendor")),
            description: None,
        })
        .unwrap();
    assert_eq!(dir.source_kind, LinkKind::Directory);
    assert_eq!(
        dir.default_project_link_path().unwrap(),
        PathBuf::from("vendor").join("shared-assets")
    );
}

#[test]
fn resource_registration_requires_project_target_directory() {
    let temp_dir = TestDir::new("resource-target-required");
    let file_source = temp_dir.path().join("notes.md");
    fs::write(&file_source, "notes").unwrap();
    let resolution = resolution(temp_dir.path());
    Store::migrate(&resolution).unwrap();

    let error = Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Resource,
            name: "notes".to_string(),
            alias: None,
            source_path: file_source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap_err();
    assert!(error.to_string().contains("--target-dir"));

    let absolute_target = Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Resource,
            name: "absolute-target".to_string(),
            alias: None,
            source_path: file_source,
            default_target_dir: Some(temp_dir.path().join("out")),
            description: None,
        })
        .unwrap_err();
    assert!(absolute_target.to_string().contains("relative"));
}

#[cfg(unix)]
#[test]
fn registry_rejects_non_utf8_source_paths() {
    let temp_dir = TestDir::new("non-utf8-source");
    let source = temp_dir.path().join(PathBuf::from(OsString::from_vec(vec![
        b'n', b'o', b't', 0xff,
    ])));
    fs::write(&source, "resource").unwrap();
    let resolution = resolution(temp_dir.path());
    Store::migrate(&resolution).unwrap();

    let error = Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Resource,
            name: "non-utf8".to_string(),
            alias: None,
            source_path: source,
            default_target_dir: Some(PathBuf::from(".agents").join("resources")),
            description: None,
        })
        .unwrap_err();

    assert!(error.to_string().contains("valid UTF-8"));
}

#[test]
fn registry_rejects_same_name_and_same_alias_conflicts() {
    let temp_dir = TestDir::new("conflicts");
    let skill_a = temp_dir.path().join("a");
    let skill_b = temp_dir.path().join("b");
    fs::create_dir(&skill_a).unwrap();
    fs::create_dir(&skill_b).unwrap();
    fs::write(skill_a.join("SKILL.md"), "a").unwrap();
    fs::write(skill_b.join("SKILL.md"), "b").unwrap();
    let resolution = resolution(temp_dir.path());
    Store::migrate(&resolution).unwrap();

    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "dup".to_string(),
            alias: Some("common".to_string()),
            source_path: skill_a,
            default_target_dir: None,
            description: None,
        })
        .unwrap();

    let same_name = Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "dup".to_string(),
            alias: Some("other".to_string()),
            source_path: skill_b.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap_err();
    assert!(
        same_name.to_string().contains("already registered")
            || same_name.to_string().contains("UNIQUE")
    );

    let same_alias = Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "unique".to_string(),
            alias: Some("common".to_string()),
            source_path: skill_b,
            default_target_dir: None,
            description: None,
        })
        .unwrap_err();
    assert!(
        same_alias.to_string().contains("link name") || same_alias.to_string().contains("UNIQUE")
    );
}

#[test]
fn rename_and_refresh_do_not_change_source_contents() {
    let temp_dir = TestDir::new("rename-refresh");
    let source = temp_dir.path().join("skill-source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("SKILL.md"), "stable").unwrap();
    let resolution = resolution(temp_dir.path());
    Store::migrate(&resolution).unwrap();

    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "old".to_string(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();

    let renamed = Store::open_rw(&resolution)
        .unwrap()
        .rename_item(LinkableItemType::Skill, "old", "new")
        .unwrap();
    assert_eq!(renamed.name, "new");
    let refreshed = Store::open_rw(&resolution)
        .unwrap()
        .refresh_item(LinkableItemType::Skill, "new")
        .unwrap();
    assert_eq!(refreshed.source_kind, LinkKind::Directory);
    assert_eq!(
        fs::read_to_string(source.join("SKILL.md")).unwrap(),
        "stable"
    );
}

#[test]
fn show_and_remove_operate_only_on_registry_rows() {
    let temp_dir = TestDir::new("show-remove");
    let source = temp_dir.path().join("skill-source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("SKILL.md"), "stable").unwrap();
    let resolution = resolution(temp_dir.path());
    Store::migrate(&resolution).unwrap();

    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "stored".to_string(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();

    assert_eq!(
        Store::open_ro(&resolution)
            .unwrap()
            .find_item(LinkableItemType::Skill, "stored")
            .unwrap()
            .source_path,
        crate::core::test_support::canonical_path(&source)
    );

    let removed = Store::open_rw(&resolution)
        .unwrap()
        .remove_item(LinkableItemType::Skill, "stored")
        .unwrap();
    assert_eq!(removed.name, "stored");
    assert!(source.join("SKILL.md").is_file());
    assert!(Store::open_ro(&resolution)
        .unwrap()
        .find_item(LinkableItemType::Skill, "stored")
        .is_err());
}

#[test]
fn refresh_reports_kind_mismatch() {
    let temp_dir = TestDir::new("refresh-kind-mismatch");
    let source = temp_dir.path().join("resource");
    fs::write(&source, "file").unwrap();
    let resolution = resolution(temp_dir.path());
    Store::migrate(&resolution).unwrap();

    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Resource,
            name: "res".to_string(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: Some(PathBuf::from(".agents").join("resources")),
            description: None,
        })
        .unwrap();

    fs::remove_file(&source).unwrap();
    fs::create_dir(&source).unwrap();
    let error = Store::open_rw(&resolution)
        .unwrap()
        .refresh_item(LinkableItemType::Resource, "res")
        .unwrap_err();
    assert!(error.to_string().contains("source kind changed"));
}

#[test]
fn list_items_is_scoped_by_type() {
    let temp_dir = TestDir::new("list");
    let skill = temp_dir.path().join("skill");
    fs::create_dir(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), "skill").unwrap();
    let resource = temp_dir.path().join("resource.md");
    fs::write(&resource, "resource").unwrap();
    let resolution = resolution(temp_dir.path());
    Store::migrate(&resolution).unwrap();

    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "skill".to_string(),
            alias: None,
            source_path: skill,
            default_target_dir: None,
            description: None,
        })
        .unwrap();
    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Resource,
            name: "resource".to_string(),
            alias: None,
            source_path: resource,
            default_target_dir: Some(PathBuf::from(".agents").join("resources")),
            description: None,
        })
        .unwrap();

    assert_eq!(
        Store::open_ro(&resolution)
            .unwrap()
            .list_items(LinkableItemType::Skill)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        Store::open_ro(&resolution)
            .unwrap()
            .list_items(LinkableItemType::Resource)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn group_lifecycle_tracks_items_without_touching_sources() {
    let temp_dir = TestDir::new("group-lifecycle");
    let source = temp_dir.path().join("skill-source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("SKILL.md"), "stable").unwrap();
    let resolution = resolution(temp_dir.path());
    Store::migrate(&resolution).unwrap();

    Store::open_rw(&resolution)
        .unwrap()
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "writer".to_string(),
            alias: None,
            source_path: source.clone(),
            default_target_dir: None,
            description: None,
        })
        .unwrap();
    let group = Store::open_rw(&resolution)
        .unwrap()
        .create_group("daily", None)
        .unwrap();
    assert_eq!(group.items.len(), 0);

    let updated = Store::open_rw(&resolution)
        .unwrap()
        .add_group_items("daily", &["writer".to_string()])
        .unwrap();
    assert_eq!(updated.group.items.len(), 1);
    assert_eq!(
        Store::open_ro(&resolution)
            .unwrap()
            .show_group("daily")
            .unwrap()
            .items[0]
            .name,
        "writer"
    );

    let duplicate = Store::open_rw(&resolution)
        .unwrap()
        .add_group_items("daily", &["writer".to_string()])
        .unwrap();
    assert_eq!(duplicate.group.items.len(), 1);
    assert_eq!(duplicate.skipped.len(), 1);

    let missing = Store::open_rw(&resolution)
        .unwrap()
        .add_group_items("daily", &["missing".to_string()])
        .unwrap_err();
    assert!(missing.to_string().contains("unknown linkable item"));

    let renamed = Store::open_rw(&resolution)
        .unwrap()
        .rename_group("daily", "workday")
        .unwrap();
    assert_eq!(renamed.name, "workday");

    let removed = Store::open_rw(&resolution)
        .unwrap()
        .remove_group_items("workday", &["writer".to_string()])
        .unwrap();
    assert!(removed.items.is_empty());
    let deleted = Store::open_rw(&resolution)
        .unwrap()
        .delete_group("workday")
        .unwrap();
    assert_eq!(deleted.name, "workday");
    assert!(source.join("SKILL.md").is_file());
    assert!(Store::open_ro(&resolution)
        .unwrap()
        .show_group("workday")
        .is_err());
}

#[test]
fn registry_identifiers_ignore_aliases_for_both_item_types() {
    for item_type in [LinkableItemType::Skill, LinkableItemType::Resource] {
        let temp = TestDir::new("identity-alias");
        let resolution = resolution(temp.path());
        Store::migrate(&resolution).unwrap();
        let store = Store::open_rw(&resolution).unwrap();
        let register = |name: &str, alias: &str| {
            let source = temp.path().join(name);
            if item_type == LinkableItemType::Skill {
                fs::create_dir(&source).unwrap();
                fs::write(source.join("SKILL.md"), name).unwrap();
            } else {
                fs::write(&source, name).unwrap();
            }
            store
                .add_item(AddLinkableItem {
                    item_type,
                    name: name.into(),
                    alias: Some(alias.into()),
                    source_path: source,
                    default_target_dir: (item_type == LinkableItemType::Resource)
                        .then(|| PathBuf::from("docs")),
                    description: None,
                })
                .unwrap()
        };
        let alpha = register("alpha", "beta");
        store.create_group("daily", None).unwrap();

        // Alias alone must fail before any mutation can select alpha.
        assert!(store.find_item(item_type, "beta").is_err());
        assert!(store.find_any_item("beta").is_err());
        assert!(store.rename_item(item_type, "beta", "delta").is_err());
        assert!(store.refresh_item(item_type, "beta").is_err());
        assert!(store.remove_item(item_type, "beta").is_err());
        assert!(store.add_group_items("daily", &["beta".into()]).is_err());
        assert!(store.remove_group_items("daily", &["beta".into()]).is_err());
        assert_eq!(store.find_item(item_type, &alpha.id).unwrap().id, alpha.id);
        assert_eq!(store.find_any_item(&alpha.id).unwrap().id, alpha.id);
        let beta = register("beta", "gamma");
        assert_eq!(store.find_item(item_type, "beta").unwrap().id, beta.id);
        assert_eq!(store.find_any_item("beta").unwrap().id, beta.id);
        assert_eq!(store.refresh_item(item_type, "beta").unwrap().id, beta.id);
        assert!(store.find_item(item_type, "gamma").is_err());
        assert!(store.find_any_item("gamma").is_err());
        assert_eq!(alpha.link_name(), "beta");
        assert_eq!(beta.link_name(), "gamma");
        let group = store
            .add_group_items("daily", &[alpha.id.clone(), "beta".into()])
            .unwrap()
            .group;
        assert_eq!(group.items.len(), 2);
        let group = store.remove_group_items("daily", &["beta".into()]).unwrap();
        assert_eq!(group.items.len(), 1);
        assert_eq!(group.items[0].id, alpha.id);
        let renamed = store.rename_item(item_type, "beta", "delta").unwrap();
        assert_eq!(renamed.id, beta.id);
        assert_eq!(renamed.name, "delta");
        assert_eq!(renamed.alias, beta.alias);
        assert_eq!(renamed.link_name(), "gamma");
        assert_eq!(
            store.rename_item(item_type, "delta", "beta").unwrap().id,
            beta.id
        );
        assert_eq!(store.remove_item(item_type, "beta").unwrap().id, beta.id);
        assert_eq!(store.find_any_item("alpha").unwrap(), alpha);
        assert!(store.find_any_item("beta").is_err());
    }
}

#[test]
fn alias_of_another_type_does_not_make_name_lookup_ambiguous() {
    let temp = TestDir::new("identity-cross-type");
    let resolution = resolution(temp.path());
    Store::migrate(&resolution).unwrap();
    let store = Store::open_rw(&resolution).unwrap();
    let skill = temp.path().join("skill");
    fs::create_dir(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), "alpha").unwrap();
    store
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Skill,
            name: "alpha".into(),
            alias: Some("beta".into()),
            source_path: skill,
            default_target_dir: None,
            description: None,
        })
        .unwrap();
    let source = temp.path().join("resource.txt");
    fs::write(&source, "beta").unwrap();
    let beta = store
        .add_item(AddLinkableItem {
            item_type: LinkableItemType::Resource,
            name: "beta".into(),
            alias: None,
            source_path: source,
            default_target_dir: Some(PathBuf::from("docs")),
            description: None,
        })
        .unwrap();

    assert_eq!(store.find_any_item("beta").unwrap().id, beta.id);
    store.create_group("daily", None).unwrap();
    assert_eq!(
        store
            .add_group_items("daily", &["beta".into()])
            .unwrap()
            .group
            .items[0]
            .id,
        beta.id
    );
}
