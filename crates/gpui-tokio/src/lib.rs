use std::future::Future;

use gpui::{App, AppContext, Global, Task};

pub use tokio::task::JoinError;

pub fn init(cx: &mut App) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("failed to initialize Tokio runtime");
    cx.set_global(GlobalTokio {
        runtime: Some(runtime),
    });
}

struct GlobalTokio {
    // shutdown_background consumes the runtime, so Drop takes it from this slot.
    runtime: Option<tokio::runtime::Runtime>,
}

impl Global for GlobalTokio {}

impl Drop for GlobalTokio {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

pub struct Tokio;

impl Tokio {
    pub fn spawn<C, Fut, R>(cx: &C, future: Fut) -> Task<Result<R, JoinError>>
    where
        C: AppContext,
        Fut: Future<Output = R> + Send + 'static,
        R: Send + 'static,
    {
        cx.read_global(|tokio: &GlobalTokio, cx| {
            let task = tokio_util::task::AbortOnDropHandle::new(
                tokio
                    .runtime
                    .as_ref()
                    .expect("Tokio runtime is initialized")
                    .spawn(future),
            );
            cx.background_spawn(task)
        })
    }
}
