//! Master / layout parse caches, keyed by archive path. PPTX presentations
//! routinely share one master across many slides; re-parsing per slide
//! would multiply XML deserialization cost. The cache stores the typed
//! [`SlideMaster`] / [`SlideLayout`] plus the master's
//! [`ColorResolver`] (which itself depends on the master's color map).

use std::collections::BTreeMap;

use slideglance_color::ColorResolver;
use slideglance_model::{SlideLayout, SlideMaster, Theme};
use slideglance_parser::{parse_slide_layout, parse_slide_master, PptxArchive};

use crate::PptxError;

pub(crate) struct CachedMaster {
    pub master: SlideMaster,
    pub color_resolver: ColorResolver,
}

pub(crate) struct CachedLayout {
    pub layout: SlideLayout,
}

#[derive(Default)]
pub(crate) struct MasterCache {
    inner: BTreeMap<String, CachedMaster>,
}

impl MasterCache {
    pub fn ensure(
        &mut self,
        master_path: &str,
        theme: &Theme,
        archive: &mut PptxArchive,
    ) -> Result<(), PptxError> {
        if self.inner.contains_key(master_path) {
            return Ok(());
        }
        let Some(master_xml) = archive.xml(master_path).map(str::to_owned) else {
            return Ok(());
        };
        // We resolve with a placeholder color resolver because the master
        // itself does not need its own colors during parse — but we need
        // a resolver to pass into placeholder lst_style resolution.
        let bootstrap_resolver =
            ColorResolver::new(theme.color_scheme, slideglance_color::ColorMap::default());
        let master = parse_slide_master(
            &master_xml,
            master_path,
            archive,
            &bootstrap_resolver,
            Some(&theme.font_scheme),
            theme.fmt_scheme.as_ref(),
        )?;
        let color_resolver = ColorResolver::new(theme.color_scheme, master.color_map);
        // Re-parse the master with the proper resolver so placeholder
        // colors and master shape colors land correctly. (parse_slide_master
        // is idempotent and cheap-ish; we pay for it once per master.)
        let master_resolved = parse_slide_master(
            &master_xml,
            master_path,
            archive,
            &color_resolver,
            Some(&theme.font_scheme),
            theme.fmt_scheme.as_ref(),
        )?;
        self.inner.insert(
            master_path.to_owned(),
            CachedMaster {
                master: master_resolved,
                color_resolver,
            },
        );
        Ok(())
    }

    pub fn get(&self, master_path: &str) -> Option<&CachedMaster> {
        self.inner.get(master_path)
    }
}

#[derive(Default)]
pub(crate) struct LayoutCache {
    inner: BTreeMap<String, CachedLayout>,
}

impl LayoutCache {
    pub fn ensure(
        &mut self,
        layout_path: &str,
        theme: &Theme,
        layout_color_resolver: &ColorResolver,
        archive: &mut PptxArchive,
    ) -> Result<(), PptxError> {
        if self.inner.contains_key(layout_path) {
            return Ok(());
        }
        let Some(layout_xml) = archive.xml(layout_path).map(str::to_owned) else {
            return Ok(());
        };
        let layout = parse_slide_layout(
            &layout_xml,
            layout_path,
            archive,
            layout_color_resolver,
            Some(&theme.font_scheme),
            theme.fmt_scheme.as_ref(),
        )?;
        self.inner
            .insert(layout_path.to_owned(), CachedLayout { layout });
        Ok(())
    }

    pub fn get(&self, layout_path: &str) -> Option<&CachedLayout> {
        self.inner.get(layout_path)
    }
}
