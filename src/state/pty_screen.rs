#[derive(Clone, Debug)]
pub enum PtyColor {
    Default,
    Rgb(u8, u8, u8),
    Indexed(u8),
}

#[derive(Clone, Debug)]
pub struct PtyCell {
    pub ch:        String,  // empty = space; may be multi-byte for wide chars
    pub fg:        PtyColor,
    pub bg:        PtyColor,
    pub bold:      bool,
    pub italic:    bool,
    pub underline: bool,
    pub reversed:  bool,
}

pub type PtyScreen = Vec<Vec<PtyCell>>;
