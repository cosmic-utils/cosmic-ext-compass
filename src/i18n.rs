// SPDX-License-Identifier: GPL-3.0-only

use i18n_embed::{
    DefaultLocalizer, LanguageLoader, Localizer,
    fluent::{FluentLanguageLoader, fluent_language_loader},
    unic_langid::LanguageIdentifier,
};
use rust_embed::RustEmbed;
use std::sync::LazyLock;

pub fn init(requested_languages: &[LanguageIdentifier]) {
    if let Err(error) = localizer().select(requested_languages) {
        tracing::warn!(%error, "could not load localization");
    }
}

#[must_use]
pub fn localizer() -> Box<dyn Localizer> {
    Box::from(DefaultLocalizer::new(&*LANGUAGE_LOADER, &Localizations))
}

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

pub static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();
    loader
        .load_fallback_language(&Localizations)
        .expect("fallback localization must load");
    loader
});

#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{ i18n_embed_fl::fl!($crate::i18n::LANGUAGE_LOADER, $message_id) }};
}
