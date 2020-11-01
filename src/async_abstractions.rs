use std::future::Future;
use tokio::runtime;

/// call only from the main GTK thread, otherwise it'll panic
/// also better dont block on the on_complete closure
pub fn spawn_future<F, C>(runtime: runtime::Handle, future: F, on_complete: Option<C>)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
    C: Fn(F::Output) + 'static,
{
    let join_handle = runtime.spawn(future);

    glib::MainContext::default().spawn_local(async move {
        // await the future execution on the other thread
        let res = join_handle.await.unwrap();

        // run the closure, if given
        if let Some(closure) = on_complete {
            closure(res);
        }
    });
}
