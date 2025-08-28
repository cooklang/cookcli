# Contributing

## Getting Started

### Some Ways to Contribute

* Report potential bugs.
* Suggest app enhancements.
* Increase our test coverage.
* Fix a [bug](https://github.com/cooklang/CookCLI/labels/bug).
* Implement a requested [enhancement](https://github.com/cooklang/CookCLI/labels/enhancement).
* Improve our documentation.
* Respond to questions about usage on the issue tracker or [Discord Server](https://discord.gg/fUVVvUzEEK).

### Reporting an Issue

> Note: Issues on GitHub for `CookCLI` are intended to be related to bugs or feature requests.
> Questions should be directed to [Discord Server](https://discord.gg/fUVVvUzEEK) or [Spec Discussions](https://github.com/cooklang/spec/discussions).

* Check existing issues (both open and closed) to make sure it has not been
reported previously.

* Provide a reproducible test case. If a contributor can't reproduce an issue,
then it dramatically lowers the chances it'll get fixed.

* Aim to respond promptly to any questions made by the `CookCLI` team on your
issue. Stale issues will be closed.

### Issue Lifecycle

1. The issue is reported.

2. The issue is verified and categorized by a `CookCLI` maintainer.
   Categorization is done via tags. For example, bugs are tagged as "bug".

3. Unless it is critical, the issue is left for a period of time (sometimes many
   weeks), giving outside contributors a chance to address the issue.

4. The issue is addressed in a pull request or commit. The issue will be
   referenced in the commit message so that the code that fixes it is clearly
   linked. Any change a `CookCLI` user might need to know about will include a
   changelog entry in the PR.

5. The issue is closed.

## Making Changes to `CookCLI`

### Prerequisites

If you wish to work on `CookCLI` itself, you'll first need to:
- install [Rust](https://www.rust-lang.org/tools/install) for macOS, Linux or Windows.
- install [NodeJS](https://nodejs.org/en/download/package-manager/) for building the web UI styles.
- fork the `CookCLI` repo

### Building `CookCLI`

To build `CookCLI`:

```bash
# Install frontend dependencies (first time only)
npm install

# Build everything including CSS
make build

# Or build just the Rust binary
cargo build
```

In a few moments, you'll have a working `cook` executable in `target/debug`.

>Note: `make build` will compile CSS and build for your local machine's os/architecture.

## Web UI Development

The web interface uses server-side rendering with Askama templates and Tailwind CSS.

### Quick Start for Frontend Development

```bash
# Setup
npm install              # Install Tailwind and dependencies
make css                 # Build CSS once

# Development workflow
npm run watch-css        # Terminal 1: Watch CSS changes
cargo run -- server      # Terminal 2: Run dev server

# Build for production
make release             # Builds everything including CSS
```

### Frontend Stack

* **Templates**: Askama templates in `templates/` directory
* **Styling**: Tailwind CSS with custom components
* **JavaScript**: Vanilla JS for interactivity (no framework dependency)
* **Static Files**: Served from `static/` directory

### Working with Templates

Templates are in `templates/` and use the Askama templating engine:

```html
<!-- templates/recipe.html -->
{% extends "base.html" %}
{% block content %}
  <h1>{{ recipe.name }}</h1>
  <!-- Recipe content -->
{% endblock %}
```

Template data structures are defined in `src/server/templates.rs`.

### Modifying Styles

The UI uses Tailwind CSS with custom components:

1. Edit styles in `static/css/input.css`
2. Run `npm run build-css` to compile
3. For development, use `npm run watch-css` for auto-rebuild

Custom component classes:
* `.recipe-card` - Recipe cards with gradient borders
* `.ingredient-badge` - Ingredient tags in recipes
* `.metadata-pill` - Clean metadata badges
* `.nav-pill` - Navigation items

### Adding New Pages

1. Create template in `templates/`
2. Define data structure in `src/server/templates.rs`
3. Add handler in `src/server/ui.rs`
4. Add route to the router

### File Structure

```
├── templates/           # Askama HTML templates
│   ├── base.html       # Base layout
│   ├── recipes.html    # Recipe listing
│   └── recipe.html     # Single recipe view
├── static/             # Static assets
│   └── css/
│       ├── input.css   # Tailwind input with custom classes
│       └── output.css  # Compiled CSS (generated)
├── src/server/
│   ├── mod.rs         # Server setup and routing
│   ├── ui.rs          # UI request handlers
│   └── templates.rs   # Template data structures
├── tailwind.config.js  # Tailwind configuration
└── package.json        # NPM dependencies
```

### Development Tips

- Templates are recompiled on each build, so you need to rebuild after template changes
- CSS changes with `npm run watch-css` are reflected immediately (just refresh browser)
- Use browser dev tools to inspect Tailwind classes
- Check `static/css/output.css` is being generated and served correctly

### Testing

No tests at the moment 🤞.

### Submitting a Pull Request

Before writing any code, we recommend:
- Create a Github issue if none already exists for the code change you'd like to make.
- Write a comment on the Github issue indicating you're interested in contributing so
maintainers can provide their perspective if needed.
- Use [Semantic Commit Messages](https://gist.github.com/joshbuchea/6f47e86d2510bce28f8e7f42ae84c716), so release automation can kick-in.

Keep your pull requests (PRs) small and open them early so you can get feedback on
approach from maintainers before investing your time in larger changes.

When you're ready to submit a pull request:
1. Include evidence that your changes work as intended (e.g., add/modify unit tests;
   describe manual tests you ran, in what environment,
   and the results including screenshots or terminal output).
2. Open the PR from your fork against base repository `cooklang/CookCLI` and branch `main`.
   - [Link the PR to its associated issue](https://docs.github.com/en/issues/tracking-your-work-with-issues/linking-a-pull-request-to-an-issue).
3. Include any specific questions that you have for the reviewer in the PR description
   or as a PR comment in Github.
   - If there's anything you find the need to explain or clarify in the PR, consider
   whether that explanation should be added in the source code as comments.
   - You can submit a [draft PR](https://github.blog/2019-02-14-introducing-draft-pull-requests/)
   if your changes aren't finalized but would benefit from in-process feedback.
6. After you submit, the `CookCLI` maintainers team needs time to carefully review your
   contribution and ensure it is production-ready, considering factors such as: correctness,
   backwards-compatibility, potential regressions, etc.
7. After you address `CookCLI` maintainer feedback and the PR is approved, a `CookCLI` maintainer
   will merge it. Your contribution will be available from the next minor release.
