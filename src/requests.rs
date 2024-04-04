use crate::pixels::Pixel;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct PlaceRequestBody {
    pub pixel: Pixel,
    pub index: u32,
}
