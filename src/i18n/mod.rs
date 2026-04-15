// Lightweight i18n backed by Fluent. Strings are embedded with `include_str!`
// so the compiled binary is fully self-contained — no runtime file IO for
// translations.
//
// Supported locales: en-US (default), ko-KR, ja-JP.

use std::borrow::Cow;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

const EN_FTL: &str = include_str!("en-US.ftl");
const KO_FTL: &str = include_str!("ko-KR.ftl");
const JA_FTL: &str = include_str!("ja-JP.ftl");

fn lang(tag: &str) -> LanguageIdentifier {
    tag.parse().expect("valid BCP-47 tag")
}

pub struct I18n {
    primary: FluentBundle<FluentResource>,
    fallback: FluentBundle<FluentResource>,
}

impl I18n {
    pub fn new(locale: &str) -> Self {
        let (tag, ftl) = match locale {
            "ko" | "ko-KR" | "kor" => ("ko-KR", KO_FTL),
            "ja" | "ja-JP" | "jpn" => ("ja-JP", JA_FTL),
            _ => ("en-US", EN_FTL),
        };
        Self {
            primary: build_bundle(lang(tag), ftl),
            fallback: build_bundle(lang("en-US"), EN_FTL),
        }
    }

    /// Format a message by id. `args` is a slice of (key, string-value) pairs.
    pub fn t(&self, id: &str, args: &[(&str, String)]) -> String {
        let mut fluent_args = FluentArgs::new();
        for (k, v) in args {
            fluent_args.set(*k, FluentValue::from(v.clone()));
        }
        let pattern = self
            .primary
            .get_message(id)
            .and_then(|m| m.value())
            .or_else(|| self.fallback.get_message(id).and_then(|m| m.value()));
        match pattern {
            Some(p) => {
                let mut errors = vec![];
                let bundle = if self.primary.get_message(id).is_some() {
                    &self.primary
                } else {
                    &self.fallback
                };
                let s: Cow<str> = bundle.format_pattern(p, Some(&fluent_args), &mut errors);
                s.into_owned()
            }
            None => id.to_string(),
        }
    }
}

fn build_bundle(langid: LanguageIdentifier, src: &str) -> FluentBundle<FluentResource> {
    let resource = FluentResource::try_new(src.to_string()).expect("valid ftl");
    let mut bundle = FluentBundle::new(vec![langid]);
    // Fluent inserts isolating Unicode marks around arguments by default which
    // looks awful in our UI. Disable.
    bundle.set_use_isolating(false);
    bundle.add_resource(resource).expect("add fluent resource");
    bundle
}

/// Detect the user's preferred locale from the OS, or fall back to en-US.
pub fn detect_locale() -> String {
    sys_locale::get_locale()
        .map(|l| {
            let l = l.replace('_', "-");
            if l.starts_with("ko") {
                "ko-KR".to_string()
            } else if l.starts_with("ja") {
                "ja-JP".to_string()
            } else {
                "en-US".to_string()
            }
        })
        .unwrap_or_else(|| "en-US".to_string())
}
