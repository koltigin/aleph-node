use codec::Decode;

use crate::{aleph_runtime::SessionKeys, Connection};

#[async_trait::async_trait]
pub trait AuthorRpc {
    async fn author_rotate_keys(&self) -> SessionKeys;
}

#[async_trait::async_trait]
impl AuthorRpc for Connection {
    async fn author_rotate_keys(&self) -> SessionKeys {
        let bytes = self.client.rpc().rotate_keys().await.unwrap();

        SessionKeys::decode(&mut bytes.0.as_slice()).unwrap()
    }
}
