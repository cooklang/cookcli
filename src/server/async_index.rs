use anyhow::Result;

use cooklang_fs::{FsIndex, RecipeEntry};

use tokio::sync::{mpsc, oneshot};

pub struct AsyncFsIndex {
    tx: mpsc::Sender<Message>,
}

pub type Responder<T> = oneshot::Sender<Result<T, cooklang_fs::Error>>;

#[derive(Debug)]
enum Message {
    Get {
        recipe: String,
        resp: Responder<RecipeEntry>,
    },
}

impl AsyncFsIndex {
    pub fn new(mut index: FsIndex) -> Result<Self> {
        index.index_all()?;

        let (tx, mut rx) = mpsc::channel::<Message>(1);

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::Get { recipe, resp } => {
                        let r = index.get(&recipe);
                        let _ = resp.send(r);
                    }
                }
            }
        });

        Ok(Self { tx })
    }

    pub async fn get(&self, recipe: String) -> Result<RecipeEntry, cooklang_fs::Error> {
        tracing::trace!("Looking up '{recipe}'");
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Message::Get { recipe, resp: tx })
            .await
            .unwrap();
        rx.await.unwrap()
    }
}
