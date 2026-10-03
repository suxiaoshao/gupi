use super::Available;
use super::Cache;
use super::Problem;
use super::Release;
use super::Status;
use super::Version;
use super::get;
use gpui_kit::Task;
use gpui_kit::TestAppContext;
use std::cell::RefCell;
use std::rc::Rc;

#[gpui_kit::test]
fn automatic_check_cancels_but_manual_check_survives_disabling(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let owner = get(cx);
        owner.update(cx, |owner, cx| {
            owner.configure(true, cx);
            owner.status = Status::Checking {
                _task: Task::ready(()),
                manual: false,
            };
            owner.configure(false, cx);
            assert!(matches!(owner.status, Status::Idle));
            assert!(owner.periodic.is_none());
            owner.configure(true, cx);
            owner.status = Status::Checking {
                _task: Task::ready(()),
                manual: false,
            };
            owner.check(true, cx); // Promotes an existing request; no Tokio runtime needed.
            owner.configure(false, cx);
            assert!(matches!(
                owner.status,
                Status::Checking { manual: true, .. }
            ));
            owner.stop(cx);
            owner.configure(true, cx);
            owner.check(true, cx);
            assert!(matches!(owner.status, Status::Idle));
            assert!(owner.periodic.is_none());
        });
    });
}

#[gpui_kit::test]
fn background_notices_are_deduplicated_and_errors_do_not_emit(cx: &mut TestAppContext) {
    let notices = Rc::new(RefCell::new(Vec::new()));
    let observed = notices.clone();
    let (owner, _subscription) = cx.update(|cx| {
        let owner = get(cx);
        let subscription = cx.subscribe(&owner, move |_, event: &Available, _| {
            observed.borrow_mut().push(event.0.version.clone());
        });
        (owner, subscription)
    });
    for _ in 0..2 {
        owner.update(cx, |owner, cx| {
            let mut cache = Cache::default();
            cache.release = Some(Release {
                version: Version::new(2, 0, 0),
                url: "https://github.com/suxiaoshao/gupi/releases/tag/v2.0.0".into(),
            });
            owner.complete(Ok(cache), cx);
        });
    }
    owner.update(cx, |owner, cx| {
        owner.complete(Err(Problem::Network), cx);
        assert!(matches!(owner.status, Status::Failed(Problem::Network)));
        owner.status = Status::Checking {
            _task: Task::ready(()),
            manual: true,
        };
        let mut cache = Cache::default();
        cache.release = Some(Release {
            version: Version::new(3, 0, 0),
            url: "https://github.com/suxiaoshao/gupi/releases/tag/v3.0.0".into(),
        });
        owner.complete(Ok(cache), cx);
    });
    assert_eq!(*notices.borrow(), vec![Version::new(2, 0, 0)]);
}

#[gpui_kit::test]
fn skipped_version_stays_available_without_background_notice(cx: &mut TestAppContext) {
    let notices = Rc::new(RefCell::new(Vec::new()));
    let observed = notices.clone();
    let (owner, _subscription) = cx.update(|cx| {
        let owner = get(cx);
        let subscription = cx.subscribe(&owner, move |_, event: &Available, _| {
            observed.borrow_mut().push(event.0.version.clone());
        });
        (owner, subscription)
    });
    owner.update(cx, |owner, cx| {
        owner.skipped = Some("2.0.0".into());
        let mut cache = Cache::default();
        cache.release = Some(Release {
            version: Version::new(2, 0, 0),
            url: "https://github.com/suxiaoshao/gupi/releases/tag/v2.0.0".into(),
        });
        owner.complete(Ok(cache), cx);
        assert!(owner.installable_release().is_some());
    });
    assert!(notices.borrow().is_empty());
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[gpui_kit::test]
fn installation_excludes_checks_and_can_retry_after_failure(cx: &mut TestAppContext) {
    cx.update(|cx| {
        get(cx).update(cx, |owner, cx| {
            owner.status = Status::Available(Release {
                version: Version::new(2, 0, 0),
                url: "https://github.com/suxiaoshao/gupi/releases/tag/v2.0.0".into(),
            });
            assert!(owner.start_install(cx).is_some());
            assert!(owner.start_install(cx).is_none());
            owner.check(true, cx); // Must not start a competing request.
            assert!(owner.is_installing());
            owner.finish_install(true, cx);
            assert!(matches!(owner.status, Status::InstallFailed(_)));
            assert!(owner.start_install(cx).is_some());
            owner.finish_install(false, cx);
            assert!(matches!(owner.status, Status::Available(_)));
            owner.stop(cx);
            assert!(owner.start_install(cx).is_none());
        });
    });
}
