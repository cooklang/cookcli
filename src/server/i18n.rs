use fluent_templates::static_loader;

static_loader! {
    pub static LOCALES = {
        locales: "./locales",
        fallback_language: "en-US",
        // Disable isolating marks for cleaner output
        customise: |bundle| bundle.set_use_isolating(false),
    };
}
