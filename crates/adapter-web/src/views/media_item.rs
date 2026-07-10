/// One media attachment as the templates render it.
pub struct MediaItem {
    pub url: String,
    /// `<video>` when true, `<img>` when false.
    pub is_video: bool,
    pub caption: String,
}
