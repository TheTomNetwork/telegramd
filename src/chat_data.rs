use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatData<T> {
    pub chatid: String,
    pub message: T,
}
