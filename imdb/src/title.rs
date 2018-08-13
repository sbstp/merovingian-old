use std::hash::{Hash, Hasher};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum TitleKind {
    Movie,
    TvMovie,
    Video,
    Short,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Title {
    pub(crate) id: u32,
    pub(crate) year: u16,
    pub(crate) runtime: u16,
    pub(crate) primary_title: String,
    pub(crate) original_title: Option<String>,
    pub(crate) kind: TitleKind,
    pub(crate) votes: u32,
}

impl Title {
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    #[inline]
    pub fn year(&self) -> i32 {
        self.year as i32
    }

    #[inline]
    pub fn runtime(&self) -> i32 {
        self.runtime as i32
    }

    #[inline]
    pub fn primary_title(&self) -> &str {
        &self.primary_title
    }

    #[inline]
    pub fn original_title(&self) -> Option<&str> {
        self.original_title.as_ref().map(|s| s.as_str())
    }

    #[inline]
    pub fn kind(&self) -> TitleKind {
        self.kind
    }

    #[inline]
    pub fn votes(&self) -> u32 {
        self.votes
    }
}

impl Hash for Title {
    #[inline]
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.id.hash(hasher)
    }
}

impl PartialEq for Title {
    #[inline]
    fn eq(&self, other: &Title) -> bool {
        self.id == other.id
    }
}

impl Eq for Title {}
