#![allow(missing_docs)]

use std::borrow::Cow;

#[derive(Debug)]
#[must_use]
pub struct Asset {
    kind: Kind,
    local_path: Vec<Cow<'static, str>>,
    hosted_path: Cow<'static, str>,
}

impl Asset {
    pub fn image() -> Builder {
        Builder::new(Kind::Image)
    }

    pub fn font() -> Builder {
        Builder::new(Kind::Image)
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[must_use]
pub enum Kind {
    Image,
    Font,
}

#[must_use]
pub struct Builder {
    asset: Asset,
}

impl Builder {
    fn new(kind: Kind) -> Self {
        Self {
            asset: Asset {
                kind,
                local_path: Vec::new(),
                hosted_path: Cow::from(""),
            },
        }
    }

    pub fn local_path<S: Into<Cow<'static, str>>>(mut self, segments: Vec<S>) -> Self {
        self.asset.local_path = segments.into_iter().map(Into::into).collect();
        self
    }

    pub fn hosted_path<S: Into<Cow<'static, str>>>(mut self, path: S) -> Self {
        self.asset.hosted_path = path.into();
        self
    }

    pub fn build(self) -> Asset {
        self.asset
    }
}
