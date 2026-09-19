use super::*;

#[test]
fn settings_version_and_round_trip() {
    let root = tempfile::tempdir().unwrap();
    let value = SettingsDocument {
        settings: Settings {
            streamlink_path: Some(
                root.path()
                    .join("path with spaces")
                    .to_string_lossy()
                    .into_owned(),
            ),
            ..Settings::default()
        },
        ..SettingsDocument::default()
    };
    assert_eq!(
        SettingsDocument::from_json(&serde_json::to_string(&value).unwrap()).unwrap(),
        value
    );
    assert_eq!(value.version, 4);
    assert_eq!(value.settings.theme, Theme::System);
    assert!(!value.settings.automatic_chat);
    assert_eq!(value.settings.default_quality, QualityPolicy::Source);
    for text in ["{}", r#"{"version":5}"#, r#"{"version":0}"#] {
        assert_eq!(
            SettingsDocument::from_json(text).unwrap_err().code,
            ErrorCode::SettingsVersion
        );
    }
    assert_eq!(
        SettingsDocument::from_json("broken").unwrap_err().code,
        ErrorCode::Settings
    );
}

#[test]
fn persist_replace_clear_and_preserve_future_versions() {
    let root = tempfile::tempdir().unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    assert!(!store.path().exists());
    for name in ["one", "two"] {
        let path = root.path().join(name).to_string_lossy().into_owned();
        store.set_streamlink_path(Some(path.clone())).unwrap();
        assert_eq!(
            SettingsStore::open(root.path())
                .unwrap()
                .snapshot()
                .streamlink_path,
            Some(path)
        );
    }
    store.set_streamlink_path(None).unwrap();
    assert_eq!(
        SettingsStore::open(root.path()).unwrap().snapshot(),
        Settings::default()
    );
    fs::write(store.path(), r#"{"version":99}"#).unwrap();
    assert!(SettingsStore::open(root.path()).is_err());
    assert_eq!(
        fs::read_to_string(store.path()).unwrap(),
        r#"{"version":99}"#
    );
}

#[test]
fn migrates_only_our_version_one_path_and_roundtrips_playback_settings() {
    let root = tempfile::tempdir().unwrap();
    let path = root
        .path()
        .join("streamlink")
        .to_string_lossy()
        .into_owned();
    let old = serde_json::json!({"version":1,"streamlinkPath":path}).to_string();
    fs::write(root.path().join("settings.json"), &old).unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    let mut next = store.snapshot();
    assert_eq!(next.streamlink_path.as_deref(), Some(path.as_str()));
    assert_eq!(fs::read_to_string(store.path()).unwrap(), old);
    next.default_quality = QualityPolicy::Audio;
    next.player = PlayerSettings {
        mode: crate::streamlink::playback::PlayerMode::Mpv,
        executable: None,
        arguments: vec!["--volume=25".into(), "literal spaces".into(), String::new()],
    };
    store.update(next.clone()).unwrap();
    assert_eq!(SettingsStore::open(root.path()).unwrap().snapshot(), next);
    let before = fs::read_to_string(store.path()).unwrap();
    next.player.arguments.push("bad\0argument".into());
    assert!(store.update(next).is_err());
    assert_eq!(fs::read_to_string(store.path()).unwrap(), before);
    assert!(
        SettingsDocument::from_json(r#"{"version":1,"streamlinkPath":null,"session":{}}"#).is_err()
    );
}

#[test]
fn version_two_migration_preserves_every_playback_preference_without_rewriting() {
    let root = tempfile::tempdir().unwrap();
    let original = serde_json::json!({"version":2,"streamlinkPath":null,"player":{"mode":"mpv","executable":null,"arguments":["--volume=20"]},"defaultQuality":"high"}).to_string();
    fs::write(root.path().join("settings.json"), &original).unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    assert_eq!(store.snapshot().default_quality, QualityPolicy::High);
    assert_eq!(store.snapshot().player.arguments, ["--volume=20"]);
    assert_eq!(fs::read_to_string(store.path()).unwrap(), original);
    store.update(store.snapshot()).unwrap();
    let persisted: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(store.path()).unwrap()).unwrap();
    assert_eq!(persisted["version"], 4);
    assert_eq!(persisted["channelOverrides"], serde_json::json!({}));
}

#[test]
fn quality_and_chat_precedence_keep_false_distinct_from_inherit() {
    let root = tempfile::tempdir().unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    let global = Settings {
        default_quality: QualityPolicy::High,
        automatic_chat: true,
        ..Settings::default()
    };
    store.update(global.clone()).unwrap();
    assert_eq!(
        store.effective("123", None).unwrap().quality,
        QualityPolicy::High
    );
    for policy in [
        QualityPolicy::Source,
        QualityPolicy::High,
        QualityPolicy::Medium,
        QualityPolicy::Low,
        QualityPolicy::Audio,
    ] {
        let saved = store
            .set_channel(SaveChannelSettingsRequest {
                broadcaster_id: "123".into(),
                overrides: ChannelOverrides {
                    notifications: None,
                    quality: Some(policy),
                    automatic_chat: Some(false),
                },
            })
            .unwrap();
        assert_eq!(saved.effective.quality, policy);
        assert!(!saved.effective.automatic_chat);
        assert_eq!(saved.default_quality, QualityPolicy::High);
        assert!(saved.default_automatic_chat);
        assert_eq!(
            store
                .effective("123", Some(QualityPolicy::Medium))
                .unwrap()
                .quality,
            QualityPolicy::Medium
        );
        assert_eq!(
            store.effective("456", None).unwrap().quality,
            QualityPolicy::High
        );
        assert!(store.effective("456", None).unwrap().automatic_chat);
    }
    store
        .set_channel(SaveChannelSettingsRequest {
            broadcaster_id: "123".into(),
            overrides: ChannelOverrides::default(),
        })
        .unwrap();
    assert!(store.value.lock().unwrap().channel_overrides.is_empty());
    assert_eq!(
        store.effective("123", None).unwrap().quality,
        QualityPolicy::High
    );
    assert!(store.effective("123", None).unwrap().automatic_chat);
}

#[test]
fn global_saves_preserve_channel_records_and_resolved_snapshots_are_immutable() {
    let root = tempfile::tempdir().unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    let mut global = store.snapshot();
    let old = store.effective("123", None).unwrap();
    let overrides = ChannelOverrides {
        notifications: None,
        quality: Some(QualityPolicy::Low),
        automatic_chat: Some(true),
    };
    store
        .set_channel(SaveChannelSettingsRequest {
            broadcaster_id: "123".into(),
            overrides: overrides.clone(),
        })
        .unwrap();
    global.default_quality = QualityPolicy::Audio;
    global.theme = Theme::Light;
    store.update(global).unwrap();
    store.set_streamlink_path(None).unwrap();
    let restored = SettingsStore::open(root.path()).unwrap();
    assert_eq!(restored.channel("123").unwrap().overrides, overrides);
    assert_eq!(
        restored.effective("456", None).unwrap().quality,
        QualityPolicy::Audio
    );
    assert_eq!(restored.snapshot().theme, Theme::Light);
    assert_eq!(old.quality, QualityPolicy::Source);
    assert!(!old.automatic_chat);
}

#[test]
fn invalid_channel_ids_are_rejected_without_writing() {
    let root = tempfile::tempdir().unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    for id in [
        "",
        "0",
        "0123",
        "display-name",
        "123/../456",
        "123\n",
        "１２３",
        "123456789012345678901234567890123",
    ] {
        assert!(store.channel(id).is_err());
        assert!(store.effective(id, None).is_err());
        assert!(
            store
                .set_channel(SaveChannelSettingsRequest {
                    broadcaster_id: id.into(),
                    overrides: ChannelOverrides::default()
                })
                .is_err()
        );
    }
    assert!(!store.path().exists());
}

#[test]
fn malformed_known_schemas_are_rejected_and_original_files_stay_intact() {
    let root = tempfile::tempdir().unwrap();
    let good = serde_json::to_value(SettingsDocument::default()).unwrap();
    let mut variants = vec![];
    for (key, bad) in [
        ("theme", serde_json::json!("purple")),
        ("automaticChat", serde_json::json!("false")),
        ("streamlinkPath", serde_json::json!("relative")),
        (
            "accessToken",
            serde_json::json!("synthetic-forbidden-field"),
        ),
    ] {
        let mut next = good.clone();
        next["settings"][key] = bad;
        variants.push(next);
    }
    let mut unknown = good.clone();
    unknown["unknown"] = serde_json::json!(true);
    variants.push(unknown);
    let mut bad_id = good.clone();
    bad_id["channelOverrides"]["login"] = serde_json::json!({"quality":null,"automaticChat":null});
    variants.push(bad_id);
    let mut bad_override = good.clone();
    bad_override["channelOverrides"]["123"] =
        serde_json::json!({"quality":"ultra","automaticChat":null});
    variants.push(bad_override);
    for value in variants {
        let text = value.to_string();
        fs::write(root.path().join("settings.json"), &text).unwrap();
        assert!(SettingsStore::open(root.path()).is_err());
        assert_eq!(
            fs::read_to_string(root.path().join("settings.json")).unwrap(),
            text
        );
    }
}

#[test]
fn persistence_failure_does_not_change_memory_and_cleans_temporary_file() {
    let root = tempfile::tempdir().unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    fs::create_dir(store.path()).unwrap();
    assert!(
        store
            .update(Settings {
                theme: Theme::Dark,
                ..Settings::default()
            })
            .is_err()
    );
    assert_eq!(store.snapshot(), Settings::default());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn bounded_file_and_channel_count_fail_without_overwriting() {
    let root = tempfile::tempdir().unwrap();
    let large = " ".repeat(MAX_SETTINGS_BYTES as usize + 1);
    fs::write(root.path().join("settings.json"), &large).unwrap();
    assert!(SettingsStore::open(root.path()).is_err());
    assert_eq!(
        fs::read_to_string(root.path().join("settings.json")).unwrap(),
        large
    );
    let mut document = SettingsDocument::default();
    for id in 1..=MAX_CHANNEL_OVERRIDES {
        document.channel_overrides.insert(
            id.to_string(),
            ChannelOverrides {
                notifications: None,
                quality: Some(QualityPolicy::Low),
                automatic_chat: None,
            },
        );
    }
    let original = serde_json::to_string(&document).unwrap();
    fs::write(root.path().join("settings.json"), &original).unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    assert!(
        store
            .set_channel(SaveChannelSettingsRequest {
                broadcaster_id: "9999".into(),
                overrides: ChannelOverrides {
                    notifications: None,
                    quality: Some(QualityPolicy::Low),
                    automatic_chat: None
                }
            })
            .is_err()
    );
    assert_eq!(fs::read_to_string(store.path()).unwrap(), original);
}

#[test]
fn phase_five_migration_is_strict_preserves_channel_preferences_and_defaults_off() {
    let root = tempfile::tempdir().unwrap();
    let streamlink = root.path().join("Stream tools/streamlink.exe");
    let player = root.path().join("Player tools/player.exe");
    let old = serde_json::json!({"version":3,"settings":{"streamlinkPath":streamlink,"player":{"mode":"custom","executable":player,"arguments":["--volume=20","literal spaces","{literal}",""]},"defaultQuality":"high","automaticChat":true,"theme":"dark"},"channelOverrides":{"123":{"quality":"audio","automaticChat":false},"456":{"quality":null,"automaticChat":true}}});
    let bytes = old.to_string();
    fs::write(root.path().join("settings.json"), &bytes).unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    let expected = Settings {
        streamlink_path: Some(streamlink.to_string_lossy().into_owned()),
        player: PlayerSettings {
            mode: crate::streamlink::playback::PlayerMode::Custom,
            executable: Some(player.to_string_lossy().into_owned()),
            arguments: vec![
                "--volume=20".into(),
                "literal spaces".into(),
                "{literal}".into(),
                String::new(),
            ],
        },
        default_quality: QualityPolicy::High,
        automatic_chat: true,
        theme: Theme::Dark,
        background: BackgroundSettings::default(),
    };
    assert_eq!(store.snapshot(), expected);
    assert_eq!(store.snapshot().background, BackgroundSettings::default());
    assert_eq!(store.snapshot().theme, Theme::Dark);
    assert_eq!(
        store.channel("123").unwrap().effective.quality,
        QualityPolicy::Audio
    );
    assert!(!store.channel("123").unwrap().effective.automatic_chat);
    assert_eq!(fs::read_to_string(store.path()).unwrap(), bytes);
    store.update(store.snapshot()).unwrap();
    let persisted: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(store.path()).unwrap()).unwrap();
    assert_eq!(persisted["version"], 4);
    let reopened = SettingsStore::open(root.path()).unwrap();
    assert_eq!(reopened.snapshot(), expected);
    for id in ["123", "456"] {
        assert_eq!(
            reopened.channel(id).unwrap().overrides,
            store.channel(id).unwrap().overrides
        );
        assert_eq!(reopened.channel(id).unwrap().overrides.notifications, None);
        assert!(!reopened.notifications(id));
    }
    // An incomplete or foreign v3 file must never be silently reset or rewritten.
    for key in ["player", "defaultQuality", "automaticChat", "theme"] {
        let mut incomplete = old.clone();
        incomplete["settings"].as_object_mut().unwrap().remove(key);
        let original = incomplete.to_string();
        fs::write(store.path(), &original).unwrap();
        assert!(SettingsStore::open(root.path()).is_err());
        assert_eq!(fs::read_to_string(store.path()).unwrap(), original);
    }
    let mut invalid = old.clone();
    invalid["settings"]["background"] = serde_json::json!({});
    assert!(SettingsDocument::from_json(&invalid.to_string()).is_err());
    let mut invalid = persisted;
    invalid["settings"]["background"]["intervalSeconds"] = serde_json::json!(1);
    assert!(SettingsDocument::from_json(&invalid.to_string()).is_err());
}

#[test]
fn notification_overrides_inherit_enable_disable_and_remain_sparse() {
    let root = tempfile::tempdir().unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    assert!(!store.notifications("123"));
    store
        .set_channel(SaveChannelSettingsRequest {
            broadcaster_id: "123".into(),
            overrides: ChannelOverrides {
                notifications: Some(true),
                ..Default::default()
            },
        })
        .unwrap();
    assert!(store.notifications("123"));
    let mut global = store.snapshot();
    global.background.notifications_enabled = true;
    store.update(global).unwrap();
    store
        .set_channel(SaveChannelSettingsRequest {
            broadcaster_id: "123".into(),
            overrides: ChannelOverrides {
                notifications: Some(false),
                ..Default::default()
            },
        })
        .unwrap();
    assert!(!store.notifications("123"));
    assert!(store.notifications("456"));
    let restored = SettingsStore::open(root.path()).unwrap();
    assert!(!restored.channel("123").unwrap().effective_notifications);
    restored
        .set_channel(SaveChannelSettingsRequest {
            broadcaster_id: "123".into(),
            overrides: ChannelOverrides::default(),
        })
        .unwrap();
    assert!(restored.notifications("123"));
    assert!(restored.value.lock().unwrap().channel_overrides.is_empty());
}
