//! Template catalogue. Each template is a short brief that gets folded into
//! the generation prompt to steer Claude toward a specific page pattern.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Template {
    #[default]
    LandingPage,
    Portfolio,
    Blog,
    Dashboard,
    Ecommerce,
    /// No template — use the user's brief verbatim.
    Custom,
}

impl Template {
    pub fn brief(self) -> &'static str {
        match self {
            Self::LandingPage => TEMPLATE_LANDING,
            Self::Portfolio => TEMPLATE_PORTFOLIO,
            Self::Blog => TEMPLATE_BLOG,
            Self::Dashboard => TEMPLATE_DASHBOARD,
            Self::Ecommerce => TEMPLATE_ECOMMERCE,
            Self::Custom => "",
        }
    }

    pub fn all() -> &'static [Template] {
        &[
            Template::LandingPage,
            Template::Portfolio,
            Template::Blog,
            Template::Dashboard,
            Template::Ecommerce,
            Template::Custom,
        ]
    }
}

pub const TEMPLATE_LANDING: &str = "\
Landing page layout: hero with headline + CTA, 3-column feature strip, \
testimonial section, final CTA. Single responsive page.";

pub const TEMPLATE_PORTFOLIO: &str = "\
Personal portfolio: compact header with nav, responsive grid of project \
cards, each with hover preview. Include an About + Contact section.";

pub const TEMPLATE_BLOG: &str = "\
Content-first blog: masthead with recent posts, two-column reading layout \
with sidebar, article pagination. Typography-forward.";

pub const TEMPLATE_DASHBOARD: &str = "\
Analytics dashboard: sidebar nav, KPI row, 2-column main region with \
charts + tables. Empty-state placeholders are fine.";

pub const TEMPLATE_ECOMMERCE: &str = "\
Storefront: header with cart badge, product grid, filter sidebar, \
single-product detail section below.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_listed_templates_have_non_empty_briefs_except_custom() {
        for t in Template::all() {
            let b = t.brief();
            if matches!(t, Template::Custom) {
                assert!(b.is_empty());
            } else {
                assert!(!b.is_empty(), "template {t:?} has empty brief");
            }
        }
    }

    #[test]
    fn default_is_landing_page() {
        assert_eq!(Template::default(), Template::LandingPage);
    }

    #[test]
    fn briefs_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for t in Template::all() {
            if !matches!(t, Template::Custom) {
                assert!(seen.insert(t.brief()), "duplicate brief for {t:?}");
            }
        }
    }
}
