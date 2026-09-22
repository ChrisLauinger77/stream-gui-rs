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
    assert_eq!(value.version, 7);
    assert_eq!(value.settings.theme, Theme::System);
    assert!(!value.settings.automatic_chat);
    assert_eq!(value.settings.default_quality, QualityPolicy::Source);
    for text in ["{}", r#"{"version":8}"#, r#"{"version":0}"#] {
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
    assert_eq!(persisted["version"], 7);
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
                    low_latency: None,
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
        low_latency: None,
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
                low_latency: None,
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
                    low_latency: None,
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
        ..Settings::default()
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
    assert_eq!(persisted["version"], 7);
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

#[test]
fn version_four_migration_preserves_released_preferences_and_defaults_phase_six() {
    let root = tempfile::tempdir().unwrap();
    // Use portable absolute paths without depending on installed executables.
    let streamlink = root.path().join("Tools ü/Streamlink/streamlink.exe");
    let player = root.path().join("Media Players ü/mpv.exe");
    let arguments = [
        "--volume=25",
        "literal spaces",
        "{braces}",
        "\"quoted\"",
        "",
    ];
    let old = serde_json::json!({"version":4,"settings":{
        "streamlinkPath":streamlink,"player":{"mode":"mpv","executable":player,"arguments":arguments},
        "defaultQuality":"audio","automaticChat":true,"theme":"dark",
        "background":{"monitoringEnabled":true,"notificationsEnabled":true,"closeToBackground":true,"intervalSeconds":300}
    },"channelOverrides":{"123":{"quality":"low","automaticChat":false,"notifications":false},"456":{"quality":null,"automaticChat":null,"notifications":null},"789":{"quality":"high","automaticChat":true,"notifications":true}}});
    let original = old.to_string();
    fs::write(root.path().join("settings.json"), &original).unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    let settings = store.snapshot();
    assert_eq!(settings.discovery_language, None);
    assert_eq!(settings.text_scale, TextScale::Normal);
    assert!(!settings.low_latency);
    assert_eq!(settings.streamlink_path.as_deref(), streamlink.to_str());
    assert_eq!(settings.player.executable.as_deref(), player.to_str());
    assert_eq!(
        settings.player.mode,
        crate::streamlink::playback::PlayerMode::Mpv
    );
    assert_eq!(settings.player.arguments, arguments);
    assert_eq!(settings.default_quality, QualityPolicy::Audio);
    assert!(settings.automatic_chat && settings.background.close_to_background);
    assert!(settings.background.monitoring_enabled && settings.background.notifications_enabled);
    assert_eq!(settings.background.interval_seconds, 300);
    assert_eq!(settings.theme, Theme::Dark);
    let check_channels = |store: &SettingsStore| {
        for (id, quality, enabled) in [
            ("123", QualityPolicy::Low, false),
            ("789", QualityPolicy::High, true),
        ] {
            let channel = store.channel(id).unwrap();
            assert_eq!(
                channel.overrides,
                ChannelOverrides {
                    quality: Some(quality),
                    automatic_chat: Some(enabled),
                    notifications: Some(enabled),
                    low_latency: None,
                }
            );
            assert_eq!(channel.effective.quality, quality);
            assert_eq!(channel.effective.automatic_chat, enabled);
            assert!(!channel.effective.low_latency);
            assert_eq!(store.notifications(id), enabled);
        }
        let value = store.value.lock().unwrap();
        assert_eq!(value.channel_overrides.len(), 2);
        assert!(!value.channel_overrides.contains_key("456"));
    };
    check_channels(&store);
    assert_eq!(fs::read_to_string(store.path()).unwrap(), original);
    store.update(settings.clone()).unwrap();
    let reopened = SettingsStore::open(root.path()).unwrap();
    assert_eq!(reopened.snapshot(), settings);
    check_channels(&reopened);
    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(store.path()).unwrap()).unwrap();
    assert_eq!(saved["version"], 7);
    assert!(saved["settings"]["discoveryLanguage"].is_null());
    assert_eq!(saved["settings"]["lowLatency"], false);
    assert_eq!(saved["settings"]["textScale"], "100");
    for key in ["lowLatency", "textScale", "discoveryLanguage"] {
        let mut invalid = old.clone();
        invalid["settings"][key] = serde_json::json!(false);
        assert!(SettingsDocument::from_json(&invalid.to_string()).is_err());
    }
}

#[test]
fn phase_six_values_are_closed_and_invalid_values_do_not_overwrite() {
    let root = tempfile::tempdir().unwrap();
    let good = serde_json::to_value(SettingsDocument::default()).unwrap();
    for (key, bad) in [
        ("discoveryLanguage", serde_json::json!("en&first=100")),
        ("discoveryLanguage", serde_json::json!("xx")),
        ("textScale", serde_json::json!("200")),
        ("textScale", serde_json::json!(125)),
        ("lowLatency", serde_json::json!("on")),
    ] {
        let mut value = good.clone();
        value["settings"][key] = bad;
        let original = value.to_string();
        fs::write(root.path().join("settings.json"), &original).unwrap();
        assert!(SettingsStore::open(root.path()).is_err());
        assert_eq!(
            fs::read_to_string(root.path().join("settings.json")).unwrap(),
            original
        );
    }
    for language in [None, Some(StreamLanguage::En), Some(StreamLanguage::Other)] {
        for text_scale in [TextScale::Normal, TextScale::Large, TextScale::Largest] {
            let mut value = SettingsDocument::default();
            value.settings.discovery_language = language;
            value.settings.text_scale = text_scale;
            assert_eq!(
                SettingsDocument::from_json(&serde_json::to_string(&value).unwrap()).unwrap(),
                value
            );
        }
    }
}

#[test]
fn low_latency_precedence_is_sparse_and_snapshots_are_immutable() {
    let root = tempfile::tempdir().unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    let original = store.effective("123", None).unwrap();
    assert!(!original.low_latency);
    store
        .update(Settings {
            low_latency: true,
            ..Settings::default()
        })
        .unwrap();
    for (choice, expected) in [(None, true), (Some(true), true), (Some(false), false)] {
        let saved = store
            .set_channel(SaveChannelSettingsRequest {
                broadcaster_id: "123".into(),
                overrides: ChannelOverrides {
                    low_latency: choice,
                    ..Default::default()
                },
            })
            .unwrap();
        assert_eq!(saved.effective.low_latency, expected);
        assert!(saved.default_low_latency);
        assert!(!original.low_latency);
    }
    store
        .set_channel(SaveChannelSettingsRequest {
            broadcaster_id: "123".into(),
            overrides: ChannelOverrides::default(),
        })
        .unwrap();
    assert!(store.value.lock().unwrap().channel_overrides.is_empty());
    assert!(
        SettingsStore::open(root.path())
            .unwrap()
            .effective("123", None)
            .unwrap()
            .low_latency
    );
}

#[test]
fn concurrent_global_and_channel_saves_preserve_sparse_precedence_in_both_orders() {
    use std::sync::Barrier;

    for channel_first in [false, true] {
        for inherit in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let store = SettingsStore::open(root.path()).unwrap();
            let explicit = ChannelOverrides {
                quality: Some(QualityPolicy::Low),
                automatic_chat: Some(false),
                notifications: Some(false),
                low_latency: Some(false),
            };
            for id in ["123", "456"] {
                store
                    .set_channel(SaveChannelSettingsRequest {
                        broadcaster_id: id.into(),
                        overrides: explicit.clone(),
                    })
                    .unwrap();
            }
            let mut global = store.snapshot();
            global.default_quality = QualityPolicy::High;
            global.automatic_chat = true;
            global.low_latency = true;
            global.background.notifications_enabled = true;
            let overrides = if inherit {
                ChannelOverrides::default()
            } else {
                explicit.clone()
            };
            let started = Barrier::new(2);
            let first_done = Barrier::new(2);
            std::thread::scope(|scope| {
                scope.spawn(|| {
                    started.wait();
                    if channel_first {
                        first_done.wait();
                    }
                    store.update(global.clone()).unwrap();
                    if !channel_first {
                        first_done.wait();
                    }
                });
                scope.spawn(|| {
                    started.wait();
                    if !channel_first {
                        first_done.wait();
                    }
                    store
                        .set_channel(SaveChannelSettingsRequest {
                            broadcaster_id: "123".into(),
                            overrides: overrides.clone(),
                        })
                        .unwrap();
                    if channel_first {
                        first_done.wait();
                    }
                });
            });
            let restored = SettingsStore::open(root.path()).unwrap();
            assert_eq!(restored.snapshot(), global);
            assert_eq!(restored.channel("456").unwrap().overrides, explicit);
            let channel = restored.channel("123").unwrap();
            assert_eq!(channel.overrides, overrides);
            assert_eq!(
                channel.effective.quality,
                if inherit {
                    QualityPolicy::High
                } else {
                    QualityPolicy::Low
                }
            );
            assert_eq!(channel.effective.automatic_chat, inherit);
            assert_eq!(channel.effective.low_latency, inherit);
            assert_eq!(channel.effective_notifications, inherit);
            let document: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(store.path()).unwrap()).unwrap();
            assert_eq!(document["channelOverrides"].get("123").is_none(), inherit);
        }
    }
}

#[test]
fn released_schema_five_migration_preserves_all_values_and_starts_without_profiles() {
    let root = tempfile::tempdir().unwrap();
    let old = serde_json::json!({"version":5,"settings":{
        "streamlinkPath": root.path().join("stream link"),
        "player":{"mode":"custom","executable":root.path().join("播放器"),"arguments":["--volume=20", "{literal}", ""]},
        "defaultQuality":"medium","automaticChat":true,"theme":"dark",
        "background":{"monitoringEnabled":true,"notificationsEnabled":true,"closeToBackground":true,"intervalSeconds":300},
        "discoveryLanguage":"de","lowLatency":true,"textScale":"150"
    },"channelOverrides":{"123":{"quality":"audio","automaticChat":false,"notifications":false,"lowLatency":false}}});
    let text = old.to_string();
    fs::write(root.path().join("settings.json"), &text).unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    assert_eq!(fs::read_to_string(store.path()).unwrap(), text);
    let current = store.snapshot();
    assert!(current.profiles.is_empty());
    assert!(current.selected_profile_id.is_none());
    assert_eq!(current.chat_provider, ChatProvider::Browser);
    assert_eq!(current.chatterino_path, None);
    let migrated = serde_json::to_value(&current).unwrap();
    for (key, value) in old["settings"].as_object().unwrap() {
        assert_eq!(&migrated[key], value, "{key}");
    }
    store.update(current).unwrap();
    let persisted: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(store.path()).unwrap()).unwrap();
    assert_eq!(persisted["channelOverrides"], old["channelOverrides"]);
    assert_eq!(persisted["version"], 7);
    let mut hostile = old;
    hostile["settings"]["profiles"] = serde_json::json!([]);
    assert!(SettingsDocument::from_json(&hostile.to_string()).is_err());
}

fn profile(name: &str) -> profiles::ProfileDraft {
    profiles::ProfileDraft {
        name: name.into(),
        player: PlayerSettings {
            arguments: vec!["--volume=20".into()],
            ..Default::default()
        },
        quality: Some(QualityPolicy::Medium),
        low_latency: Some(true),
    }
}
#[test]
fn profile_crud_precedence_and_deletion_preserve_immutable_snapshots() {
    use profiles::ProfileMutation::*;
    let root = tempfile::tempdir().unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    let created = store
        .modify_profile(Create {
            profile: profile("Desk"),
        })
        .unwrap();
    let id = created.profiles[0].id.clone();
    assert!(created.selected_profile_id.is_none());
    store
        .modify_profile(Select {
            id: Some(id.clone()),
        })
        .unwrap();
    let first = store.effective("123", None).unwrap();
    assert_eq!(first.profile_id.as_ref(), Some(&id));
    assert_eq!(first.player.arguments, ["--volume=20"]);
    assert_eq!(first.quality, QualityPolicy::Medium);
    assert!(first.low_latency);
    for quality in [None, Some(QualityPolicy::Audio), Some(QualityPolicy::High)] {
        for low in [None, Some(false), Some(true)] {
            store
                .set_channel(SaveChannelSettingsRequest {
                    broadcaster_id: "123".into(),
                    overrides: ChannelOverrides {
                        quality,
                        low_latency: low,
                        ..Default::default()
                    },
                })
                .unwrap();
            for request in [None, Some(QualityPolicy::Low), Some(QualityPolicy::Source)] {
                let effective = store.effective("123", request).unwrap();
                assert_eq!(
                    effective.quality,
                    request.or(quality).unwrap_or(QualityPolicy::Medium)
                );
                assert_eq!(effective.low_latency, low.unwrap_or(true));
            }
        }
    }
    let mut changed = profile("Renamed");
    changed.player.arguments.clear();
    changed.quality = None;
    changed.low_latency = None;
    store
        .modify_profile(Update {
            id: id.clone(),
            profile: changed,
        })
        .unwrap();
    assert_eq!(
        store.effective("456", None).unwrap().quality,
        QualityPolicy::Source
    );
    assert!(!store.effective("456", None).unwrap().low_latency);
    assert_eq!(store.snapshot().profiles[0].id, id);
    store.modify_profile(Delete { id: id.clone() }).unwrap();
    assert!(store.snapshot().selected_profile_id.is_none());
    assert!(store.effective("456", None).unwrap().profile_id.is_none());
    assert_eq!(first.player.arguments, ["--volume=20"]);
    assert!(first.low_latency);
    assert!(
        store
            .modify_profile(Select {
                id: Some(id.clone())
            })
            .is_err()
    );
    assert!(
        store
            .modify_profile(Update {
                id,
                profile: profile("Gone")
            })
            .is_err()
    );
    assert_eq!(
        SettingsStore::open(root.path()).unwrap().snapshot(),
        store.snapshot()
    );
}
#[test]
fn profile_bounds_duplicates_and_corruption_are_rejected_without_overwrite() {
    use profiles::ProfileMutation::*;
    let root = tempfile::tempdir().unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    store
        .modify_profile(Create {
            profile: profile("Desk"),
        })
        .unwrap();
    for name in [
        "",
        " Desk",
        "desk",
        "DESK",
        "a\nb",
        &"x".repeat(65),
        &"界".repeat(64),
    ] {
        let before = fs::read(store.path()).unwrap();
        assert!(
            store
                .modify_profile(Create {
                    profile: profile(name)
                })
                .is_err(),
            "{name:?}"
        );
        assert_eq!(fs::read(store.path()).unwrap(), before);
    }
    let mut document = store.value.lock().unwrap().clone();
    document
        .settings
        .profiles
        .push(document.settings.profiles[0].clone());
    assert!(SettingsDocument::from_json(&serde_json::to_string(&document).unwrap()).is_err());
    document.settings.profiles.pop();
    document.settings.selected_profile_id = Some(uuid::Uuid::new_v4().to_string());
    assert!(SettingsDocument::from_json(&serde_json::to_string(&document).unwrap()).is_err());
    for index in 1..profiles::MAX_PROFILES {
        store
            .modify_profile(Create {
                profile: profile(&format!("Profile {index}")),
            })
            .unwrap();
    }
    assert!(
        store
            .modify_profile(Create {
                profile: profile("overflow")
            })
            .is_err()
    );
    let mut invalid = profile("bad args");
    invalid.player.arguments = vec!["bad\0arg".into()];
    let id = store.snapshot().profiles[0].id.clone();
    assert!(
        store
            .modify_profile(Update {
                id,
                profile: invalid
            })
            .is_err()
    );
}

#[test]
fn atomic_global_updates_cannot_replace_profiles_or_restore_a_deleted_selection() {
    use profiles::ProfileMutation::*;
    let root = tempfile::tempdir().unwrap();
    let store = SettingsStore::open(root.path()).unwrap();
    let stale_empty = store.snapshot();
    let created = store
        .modify_profile(Create {
            profile: profile("Keep"),
        })
        .unwrap();
    let id = created.profiles[0].id.clone();
    store
        .modify_profile(Select {
            id: Some(id.clone()),
        })
        .unwrap();
    let stale_selected = store.snapshot();
    store.update(stale_empty).unwrap();
    assert_eq!(store.snapshot().selected_profile_id, Some(id.clone()));
    store.modify_profile(Delete { id }).unwrap();
    store.update(stale_selected).unwrap();
    assert!(store.snapshot().profiles.is_empty());
    assert!(store.snapshot().selected_profile_id.is_none());
}

#[test]
fn profile_mutation_dtos_reject_unknown_execution_fields() {
    for value in [
        serde_json::json!({"kind":"select","id":null,"url":"https://evil.invalid"}),
        serde_json::json!({"kind":"delete","id":"id","arguments":["--token=synthetic"]}),
        serde_json::json!({"kind":"create","profile":{"name":"Example","player":{"mode":"default","executable":null,"arguments":[],"environment":{}},"quality":null,"lowLatency":null}}),
        serde_json::json!({"kind":"execute","executable":"anything"}),
    ] {
        assert!(serde_json::from_value::<profiles::ProfileMutation>(value).is_err());
    }
}

#[test]
fn version_six_migrates_in_memory_and_rejects_smuggled_phase_eight_fields() {
    let dir = tempfile::tempdir().unwrap();
    let mut old = serde_json::to_value(SettingsDocument::default()).unwrap();
    old["version"] = 6.into();
    let settings = old["settings"].as_object_mut().unwrap();
    settings.remove("discovery");
    settings.remove("shortcuts");
    let original = serde_json::to_string(&old).unwrap();
    fs::write(dir.path().join("settings.json"), &original).unwrap();
    let store = SettingsStore::open(dir.path()).unwrap();
    assert_eq!(
        store.snapshot().discovery,
        discovery::DiscoveryPreferences::default()
    );
    assert_eq!(fs::read_to_string(store.path()).unwrap(), original);
    old["settings"]["discovery"] = serde_json::json!({"bookmarks":[],"hidden":[]});
    assert!(SettingsDocument::from_json(&old.to_string()).is_err());
    store
        .set_shortcuts(shortcuts::ShortcutBindings::default())
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&fs::read_to_string(store.path()).unwrap())
            .unwrap()["version"],
        7
    );
}

#[test]
fn discovery_mutations_are_idempotent_bounded_and_independent_of_global_drafts() {
    use discovery::*;
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::open(dir.path()).unwrap();
    let draft = store.snapshot();
    let item = SavedItem {
        kind: ItemKind::Channel,
        id: "123".into(),
        name: "Synthetic channel".into(),
    };
    let add = DiscoveryMutation {
        list: DiscoveryList::Bookmarks,
        item: item.clone(),
        present: true,
    };
    store.modify_discovery(add.clone()).unwrap();
    store.modify_discovery(add).unwrap();
    store
        .modify_discovery(DiscoveryMutation {
            list: DiscoveryList::Hidden,
            item: item.clone(),
            present: true,
        })
        .unwrap();
    store.update(draft).unwrap();
    assert_eq!(store.snapshot().discovery.bookmarks, [item.clone()]);
    assert_eq!(store.snapshot().discovery.hidden, [item.clone()]);
    for id in 1..200 {
        store
            .modify_discovery(DiscoveryMutation {
                list: DiscoveryList::Bookmarks,
                item: SavedItem {
                    id: (1000 + id).to_string(),
                    ..item.clone()
                },
                present: true,
            })
            .unwrap();
    }
    let before = fs::read(store.path()).unwrap();
    assert!(
        store
            .modify_discovery(DiscoveryMutation {
                list: DiscoveryList::Bookmarks,
                item: SavedItem {
                    id: "9999".into(),
                    ..item.clone()
                },
                present: true
            })
            .is_err()
    );
    assert_eq!(fs::read(store.path()).unwrap(), before);
    let remove = DiscoveryMutation {
        list: DiscoveryList::Bookmarks,
        item: item.clone(),
        present: false,
    };
    store.modify_discovery(remove.clone()).unwrap();
    store.modify_discovery(remove).unwrap();
    assert_eq!(store.snapshot().discovery.bookmarks.len(), 199);
    assert_eq!(store.snapshot().discovery.hidden, [item]);
}

#[test]
fn malformed_saved_items_and_duplicate_bindings_are_rejected() {
    use discovery::*;
    let item = SavedItem {
        kind: ItemKind::Category,
        id: "123".into(),
        name: "Synthetic category".into(),
    };
    for bad in [
        SavedItem {
            id: "0".into(),
            ..item.clone()
        },
        SavedItem {
            id: "../1".into(),
            ..item.clone()
        },
        SavedItem {
            name: "bad\nname".into(),
            ..item.clone()
        },
        SavedItem {
            name: "x".repeat(257),
            ..item.clone()
        },
    ] {
        assert!(
            DiscoveryPreferences {
                bookmarks: vec![bad],
                hidden: vec![]
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        DiscoveryPreferences {
            bookmarks: vec![item.clone(), item],
            hidden: vec![]
        }
        .validate()
        .is_err()
    );
    for mac in [false, true] {
        use shortcuts::*;
        let defaults = ShortcutBindings::defaults(mac);
        defaults.validate().unwrap();
        let mut bindings = defaults.clone();
        bindings.0.insert(
            ShortcutAction::Refresh,
            bindings.0[&ShortcutAction::Search].clone(),
        );
        assert!(bindings.validate().is_err());
        let mut bindings = defaults.clone();
        bindings.0.insert(ShortcutAction::Search, None);
        bindings.validate().unwrap();
        let mut bindings = defaults.clone();
        bindings.0.remove(&ShortcutAction::Search);
        assert!(bindings.validate().is_err());
        for key in ["Shift", "Escape", "q", "../x", "K", "é", ""] {
            let mut bindings = defaults.clone();
            bindings
                .0
                .get_mut(&ShortcutAction::Search)
                .unwrap()
                .as_mut()
                .unwrap()
                .key = key.into();
            assert!(bindings.validate().is_err());
        }
    }
}

#[test]
fn discovery_shortcuts_profiles_and_channel_mutations_preserve_each_other() {
    use discovery::*;
    use shortcuts::*;
    let dir = tempfile::tempdir().unwrap();
    let store = std::sync::Arc::new(SettingsStore::open(dir.path()).unwrap());
    let draft = store.snapshot();
    let workers: Vec<_> = (0..4)
        .map(|n| {
            let store = store.clone();
            std::thread::spawn(move || match n {
                0 => {
                    store
                        .modify_discovery(DiscoveryMutation {
                            list: DiscoveryList::Hidden,
                            item: SavedItem {
                                kind: ItemKind::Channel,
                                id: "123".into(),
                                name: "Synthetic".into(),
                            },
                            present: true,
                        })
                        .unwrap();
                }
                1 => {
                    store
                        .modify_profile(ProfileMutation::Create {
                            profile: profiles::ProfileDraft {
                                name: "Synthetic".into(),
                                player: Default::default(),
                                quality: None,
                                low_latency: None,
                            },
                        })
                        .unwrap();
                }
                2 => {
                    let mut bindings = ShortcutBindings::default();
                    bindings.0.insert(ShortcutAction::Search, None);
                    store.set_shortcuts(bindings).unwrap();
                }
                _ => {
                    store
                        .set_channel(SaveChannelSettingsRequest {
                            broadcaster_id: "123".into(),
                            overrides: ChannelOverrides {
                                low_latency: Some(true),
                                ..Default::default()
                            },
                        })
                        .unwrap();
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    store.update(draft).unwrap();
    let restored = SettingsStore::open(dir.path()).unwrap();
    assert_eq!(restored.snapshot().discovery.hidden.len(), 1);
    assert_eq!(restored.snapshot().profiles.len(), 1);
    assert_eq!(
        restored.snapshot().shortcuts.0[&ShortcutAction::Search],
        None
    );
    assert_eq!(
        restored.channel("123").unwrap().overrides.low_latency,
        Some(true)
    );
}
