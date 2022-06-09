use anyhow::bail;
use etcd_client::{Client, Compare, CompareOp, EventType, Txn, TxnOp, TxnOpResponse};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};

// superposition

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::time::sleep;

use tracing::{debug, error, trace, warn};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Stored<T: Clone + Send + Sync + Default + 'static> {
    ident: (String, String),
    inner: T,
}
impl<T: Serialize + DeserializeOwned + Clone + Send + Sync + Default + 'static> Stored<T> {
    async fn read(mut client: Client, ident: (String, String)) -> anyhow::Result<(i64, Self)> {
        trace!(
            namespace = ident.0.as_str(),
            id = ident.1.as_str(),
            "loading quark object"
        );
        let resp = client
            .get(format!("/{}/quarks/{}", ident.0, ident.1), None)
            .await?;
        if let Some(kv) = resp.kvs().first() {
            return Ok((kv.version(), serde_json::from_slice(kv.value())?));
        }
        bail!("key not found")
    }
    async fn write(&self, mut client: Client, version: i64) -> anyhow::Result<()> {
        trace!(
            namespace = self.ident.0.as_str(),
            id = self.ident.1.as_str(),
            "writing object to datastore"
        );

        let key = format!("/{}/quarks/{}", self.ident.0, self.ident.1);

        let txn = Txn::new()
            .when(vec![Compare::version(
                key.clone(),
                CompareOp::Equal,
                version,
            )])
            // update the object
            .and_then(vec![TxnOp::put(
                key.as_str(),
                serde_json::to_string(self)?,
                None,
            )]);

        trace!(
            namespace = self.ident.0.as_str(),
            id = self.ident.1.as_str(),
            "using the following transaction: {:?}",
            &txn
        );
        let resp = client.txn(txn).await?;
        trace!("transaction response: {:?}", resp);

        // extract the new version number

        Ok(())
    }
    async fn refresh(&mut self, mut client: Client, version: i64) -> anyhow::Result<()> {
        trace!(
            namespace = self.ident.0.as_str(),
            id = self.ident.1.as_str(),
            "refreshing contents"
        );
        let key = format!("/{}/quarks/{}", self.ident.0, self.ident.1);
        let txn = Txn::new()
            .when(vec![Compare::version(
                key.clone(),
                CompareOp::Equal,
                version,
            )])
            .or_else(vec![TxnOp::get(key, None)]);

        trace!(
            namespace = self.ident.0.as_str(),
            id = self.ident.1.as_str(),
            "using the following transaction: {:?}",
            &txn
        );
        let resp = client.txn(txn).await?;
        if !resp.succeeded() {
            warn!(
                namespace = self.ident.0.as_str(),
                id = self.ident.1.as_str(),
                "local version does not match remote- refreshing contents"
            );
            match &resp.op_responses()[0] {
                TxnOpResponse::Get(body) => {
                    if let Some(kv) = body.kvs().first() {
                        self.update_inner(kv.value())?;
                    }
                }
                _ => bail!("unexpected operation returned for transaction response"),
            }
        } else {
            trace!(
                namespace = self.ident.0.as_str(),
                id = self.ident.1.as_str(),
                "local & remote versions are in sync"
            )
        }

        Ok(())
    }

    fn update_inner(&mut self, buf: &[u8]) -> anyhow::Result<()> {
        *self = serde_json::from_slice(buf)?;
        Ok(())
    }
}
impl<T: Serialize + DeserializeOwned + Clone + Send + Sync + Default + 'static> Default
    for Stored<T>
{
    fn default() -> Self {
        Self {
            ident: ("".to_string(), "".to_string()),
            inner: Default::default(),
        }
    }
}

#[derive(Clone)]
struct Quark<T: Serialize + DeserializeOwned + Clone + Send + Sync + Default + 'static> {
    client: Client,
    /// (Namespace, ID)
    ident: (String, String),
    version: Arc<Mutex<i64>>,

    inner: Arc<RwLock<Stored<T>>>,
}
impl<T: Serialize + DeserializeOwned + Clone + Send + Sync + Default + 'static> Quark<T> {
    async fn new(
        client: Client,
        namespace: String,
        id: Option<String>,
        initial: Option<T>,
    ) -> anyhow::Result<Self> {
        let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let data = if let Ok(result) =
            Stored::read(client.clone(), (namespace.clone(), id.clone())).await
        {
            result
        } else {
            debug!(
                namespace = namespace.as_str(),
                id = id.as_str(),
                "object doesn't exist in remote storage, creating"
            );
            // TODO.md we need to extract the error and make sure the key doesn't exist, or handle an actual error
            let new = Stored {
                ident: (namespace.clone(), id.clone()),
                inner: initial.unwrap_or_default(),
            };
            new.write(client.clone(), 0).await?;

            (0, new)
        };
        debug!(
            namespace = namespace.as_str(),
            id = id.as_str(),
            "loaded object with version {}",
            &data.0
        );
        let version = Arc::new(Mutex::new(data.0));
        let inner = Arc::new(RwLock::new(data.1));

        let _watcher = tokio::task::spawn(quark_watcher(
            client.clone(),
            (namespace.clone(), id.clone()),
            version.clone(),
            inner.clone(),
        ));

        Ok(Self {
            client,
            ident: (namespace, id),
            version,
            inner,
        })
    }

    /// Return a reference to the contents of the Quark
    async fn lazy_read(&self) -> anyhow::Result<T> {
        let handle = self.inner.read().await;
        let contents = handle.inner.clone();
        drop(handle);
        Ok(contents)
    }

    async fn read(&mut self) -> anyhow::Result<T> {
        let version = self.version.lock().await;
        let mut inner = self.inner.write().await;

        inner.refresh(self.client.clone(), *version).await?;

        drop(inner);
        drop(version);

        self.lazy_read().await
    }

    async fn update<M: FnOnce(&mut T)>(&mut self, closure: M) -> anyhow::Result<()> {
        let lock_key = format!("{}/quarks/{}_lock", self.ident.0, self.ident.1);
        trace!("setting global object lock");
        self.client.lock(lock_key.clone(), None).await?;

        let mut version = self.version.lock().await;
        let mut handle = self.inner.write().await;

        // execute the closure, where the updates will happen
        closure(&mut handle.inner);
        // once the closure is done, keep the write handle and save the contents
        handle.write(self.client.clone(), *version).await?;

        // This updates the inner contents, but does not modify the external version stored in self
        // the etcd transaction does not return the new version, so that leaves us with either
        // guessing, or just assuming the event watcher will get the updated values.
        // I think it's safe to just increment the local value by one before we unlock the global lock

        *version += 1;

        drop(handle);
        drop(version);

        trace!("removing global object lock");
        self.client.unlock(lock_key.clone()).await?;

        Ok(())
    }
}
impl<T: Serialize + DeserializeOwned + Clone + Default + Send + Sync + Debug> Debug for Quark<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Quark")
            .field("namespace", &self.ident.0)
            .field("id", &self.ident.1)
            .field("version", &self.version)
            .field("inner", &self.inner)
            .finish()
    }
}

async fn quark_watcher<
    T: Serialize + DeserializeOwned + Clone + Send + Sync + Default + 'static,
>(
    mut client: Client,
    ident: (String, String),
    version: Arc<Mutex<i64>>,
    inner: Arc<RwLock<Stored<T>>>,
) {
    trace!("starting watcher");
    let key = format!("/{}/quarks/{}", ident.0, ident.1);
    let (_watcher, mut stream) = client.watch(key, None).await.unwrap();

    while let Some(resp) = stream.message().await.unwrap() {
        if resp.canceled() {
            // TODO.md under what case can this be canceled?
            warn!(
                namespace = ident.0.as_str(),
                id = ident.1.as_str(),
                "watcher canceled"
            );
            break;
        }

        trace!(
            namespace = ident.0.as_str(),
            id = ident.1.as_str(),
            "received watch event. {:?}",
            &resp
        );

        for event in resp.events() {
            match event.event_type() {
                EventType::Delete => {
                    // TODO.md Handle deletions
                    warn!(
                        namespace = ident.0.as_str(),
                        id = ident.1.as_str(),
                        "node has been deleted. making internal node as deleted",
                    );
                    // watcher.cancel().await.unwrap();
                }
                EventType::Put => {
                    if let Some(kv) = event.kv() {
                        let mut vers = version.lock().await;
                        if kv.version() != *vers {
                            trace!(
                                new = kv.version(),
                                old = *vers,
                                "remote version has changed, refreshing inner contents"
                            );

                            let mut r = inner.write().await;
                            match r.update_inner(kv.value()) {
                                Ok(_) => trace!("successfully updated inner contents"),
                                Err(err) => error!("unable to update inner contents. {:?}", err),
                            }

                            // now update the version
                            *vers = kv.version();

                            drop(r);
                            drop(vers);
                        }
                    } else {
                        warn!("received event with no key/value entry. ignoring")
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Testing {
    key: String,
}
impl Default for Testing {
    fn default() -> Self {
        Testing {
            key: "default".to_string(),
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let client = Client::connect(["localhost:2379"], None).await.unwrap();

    let foo = Testing {
        key: "hello world".to_string(),
    };

    let quark = Quark::new(
        client.clone(),
        "testing".to_string(),
        Some("foo2".to_string()),
        Some(foo),
    )
    .await;

    let mut quark = quark.unwrap();

    quark
        .update(|q| q.key = "updated value 3".to_string())
        .await;

    sleep(Duration::from_secs(3)).await;
}
