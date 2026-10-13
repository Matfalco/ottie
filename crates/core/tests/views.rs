#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Saved views: kept and checked by the manager, saved in the database.

use ottie_core::{
    CategoryId, CategorySettings, Error, HttpBackend, JobEvent, JobState, Manager, SortKey, Store,
    View, ViewId, ViewSettings,
};

async fn manager(store: Store) -> Manager {
    Manager::start(HttpBackend::new().unwrap(), store)
        .await
        .unwrap()
}

#[tokio::test]
async fn views_are_added_changed_and_removed_with_events() {
    let manager = manager(Store::open_in_memory().await.unwrap()).await;
    let mut events = manager.subscribe();
    assert_eq!(manager.views().await.unwrap(), []);

    let settings = ViewSettings {
        states: Some(vec![JobState::Failed, JobState::Retrying]),
        ..ViewSettings::named(" Trouble ")
    };
    let id = manager.add_view(settings).await.unwrap();
    assert_eq!(id, ViewId(1));
    let expected = View {
        id,
        settings: ViewSettings {
            states: Some(vec![JobState::Retrying, JobState::Failed]),
            ..ViewSettings::named("Trouble")
        },
    };
    assert_eq!(
        manager.views().await.unwrap(),
        std::slice::from_ref(&expected)
    );
    assert_eq!(
        events.recv().await.unwrap(),
        JobEvent::ViewChanged(expected)
    );

    let renamed = ViewSettings {
        sort: SortKey::Name,
        ..ViewSettings::named("Problems")
    };
    manager.update_view(id, renamed.clone()).await.unwrap();
    let changed = View {
        id,
        settings: renamed,
    };
    assert_eq!(events.recv().await.unwrap(), JobEvent::ViewChanged(changed));

    manager.remove_view(id).await.unwrap();
    assert_eq!(events.recv().await.unwrap(), JobEvent::ViewRemoved(id));
    assert_eq!(manager.views().await.unwrap(), []);
    assert!(matches!(
        manager.remove_view(id).await,
        Err(Error::ViewNotFound(_))
    ));
}

#[tokio::test]
async fn names_are_unique_and_categories_must_exist() {
    let manager = manager(Store::open_in_memory().await.unwrap()).await;
    let first = manager
        .add_view(ViewSettings::named("Videos"))
        .await
        .unwrap();
    let second = manager
        .add_view(ViewSettings::named("Music"))
        .await
        .unwrap();
    assert!(matches!(
        manager.add_view(ViewSettings::named("VIDEOS")).await,
        Err(Error::ViewNameTaken(_))
    ));
    assert!(matches!(
        manager
            .update_view(second, ViewSettings::named("videos"))
            .await,
        Err(Error::ViewNameTaken(_))
    ));
    // A view may keep its own name in another case.
    manager
        .update_view(first, ViewSettings::named("VIDEOS"))
        .await
        .unwrap();
    assert!(matches!(
        manager.add_view(ViewSettings::named("  ")).await,
        Err(Error::InvalidViewName)
    ));
    let unknown = ViewSettings {
        categories: Some(vec![CategoryId(99)]),
        ..ViewSettings::named("Nowhere")
    };
    assert!(matches!(
        manager.add_view(unknown).await,
        Err(Error::CategoryNotFound(_))
    ));
    assert!(matches!(
        manager
            .update_view(ViewId(42), ViewSettings::named("Lost"))
            .await,
        Err(Error::ViewNotFound(_))
    ));
}

#[tokio::test]
async fn a_removed_category_is_replaced_by_general_in_views() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("ottie.db");
    let manager = manager(Store::open(&db).await.unwrap()).await;
    let videos = manager
        .add_category(CategorySettings::named("Videos"))
        .await
        .unwrap();
    let music = manager
        .add_category(CategorySettings::named("Music"))
        .await
        .unwrap();
    let both = ViewSettings {
        categories: Some(vec![videos, music]),
        ..ViewSettings::named("Media")
    };
    let id = manager.add_view(both).await.unwrap();
    let mut events = manager.subscribe();

    manager.remove_category(videos).await.unwrap();
    let expected = View {
        id,
        settings: ViewSettings {
            categories: Some(vec![CategoryId::GENERAL, music]),
            ..ViewSettings::named("Media")
        },
    };
    assert_eq!(
        events.recv().await.unwrap(),
        JobEvent::ViewChanged(expected.clone())
    );
    assert_eq!(
        events.recv().await.unwrap(),
        JobEvent::CategoryRemoved(videos)
    );

    // Saved, too.
    manager.shutdown().await.unwrap();
    drop(manager);
    let manager = self::manager(Store::open(&db).await.unwrap()).await;
    assert_eq!(manager.views().await.unwrap(), [expected]);
    manager.shutdown().await.unwrap();
}
