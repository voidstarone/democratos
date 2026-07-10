use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateForm {
    pub handle: String,
}
