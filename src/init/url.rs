use url::Url;

pub trait UrlExt: Sized {
    fn clear_query_param(self) -> Self;

    fn add_query_param<K, V>(self, key: K, value: V) -> Self
    where
        K: AsRef<str>,
        V: AsRef<str>;

    fn reset_path<S: AsRef<str>>(self, path: S) -> Self;
}

impl UrlExt for Url {
    fn clear_query_param(mut self) -> Self {
        self.query_pairs_mut().clear().finish();
        self
    }

    fn add_query_param<K, V>(mut self, key: K, value: V) -> Self
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.query_pairs_mut()
            .append_pair(key.as_ref(), value.as_ref())
            .finish();
        self
    }

    fn reset_path<S: AsRef<str>>(mut self, path: S) -> Self {
        self.set_path(path.as_ref());
        self
    }
}
