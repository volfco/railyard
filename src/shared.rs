// use serde::{Serialize};
// use serde::de::DeserializeOwned;
// use etcd_rs;
// use etcd::kv::{self, Action};
//
// struct SharedObject<T: Serialize + DeserializeOwned> {
//     key: String,
//     epoch: u32,
//     config: (chrono::Duration),
//     inner: T
// }
// impl<T: Serialize + DeserializeOwned> SharedObject<T> {
//     async fn new(client: etcd_rs::Client, path: &str) {
//         // spawn a worker to poll for updates
//     }
//
//     async fn read(&self) { }
//     async fn consistent_read(&self) { }
//     async fn dirty_read(&self) { }
//
//     async fn write(&self) { }
//     async fn update<M: FnOnce(&mut T)>(&self, closure: M) {}
// }
//
// #[cfg(test)]
// mod tests {
//     use etcd_rs::Client;
//     use crate::shared::SharedObject;
//
//     #[test]
//     fn test_read() {
//
//         struct Foo {
//             key: String
//         }
//
//         let client = Client::new(&["http://127.0.0.1:2379"], None).unwrap();
//
//         let s: SharedObject<Foo> = SharedObject::new(client, "/foo/var");
//
//     }
// }
