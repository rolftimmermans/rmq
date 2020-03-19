use tokio::sync::watch;

#[derive(Debug)]
pub(crate) struct Exchange<T> {
    pub(crate) tx: watch::Sender<T>,
    pub(crate) rx: watch::Receiver<T>,
}

impl<T: Default + Clone> Default for Exchange<T> {
    fn default() -> Self {
        let (tx, rx) = watch::channel(Default::default());
        Self { tx, rx }
    }
}
