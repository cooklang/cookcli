// This file includes a substantial portion of code from
// https://github.com/Zheoni/cooklang-chef
//
// The original code is licensed under the MIT License, a copy of which
// is provided below in addition to our project's license.
//
//

// MIT License

// Copyright (c) 2023 Francisco J. Sanchez

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use anyhow::Result;

use cooklang_fs::{LazyFsIndex, RecipeEntry};

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
    pub fn new(index: LazyFsIndex) -> Result<Self> {
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
