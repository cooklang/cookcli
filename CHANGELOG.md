# Changelog

## [0.15.1](https://github.com/cooklang/cookcli/compare/v0.15.1...v0.15.1) (2025-08-28)


### ⚠ BREAKING CHANGES

* use : as scaling factor delimiter

### Features

* add base path for shopping lists so we can lookup references ([a5c1a42](https://github.com/cooklang/cookcli/commit/a5c1a42fd3dfb1c3ca63cfb6454ca47110789853))
* add basic search ([94741e0](https://github.com/cooklang/cookcli/commit/94741e0e712ebea7bc79db41021df768dc750075))
* add cache ([91ec988](https://github.com/cooklang/cookcli/commit/91ec988b5bcbdf089c061761ce25e19b53e853a8))
* add servings ([9f539d9](https://github.com/cooklang/cookcli/commit/9f539d9c2e901f53785ecf3edbd962b47003b276))
* add support for templates ([881495f](https://github.com/cooklang/cookcli/commit/881495f8003b21fe1084b209a9428ef231632179))
* cook doctor ([d46b166](https://github.com/cooklang/cookcli/commit/d46b16607ba687501e2808703b0a9910db83a3b6))
* correct repo name ([d6a4d0d](https://github.com/cooklang/cookcli/commit/d6a4d0d24c001a6fbc0e667e591e0919c11b3b58))
* detect cycle references ([3d9144f](https://github.com/cooklang/cookcli/commit/3d9144f66af5420440921a9e52883c9a8fc09b3c))
* diplay relative path for search results ([ac7f847](https://github.com/cooklang/cookcli/commit/ac7f847ef7dba190af678d47da736da0bebd707b))
* disable testing mode ([c46bf05](https://github.com/cooklang/cookcli/commit/c46bf05e1107f57f56f5490ff8ffd74c8c15f748))
* highlight sections in shopping list ([342db6c](https://github.com/cooklang/cookcli/commit/342db6c6b2ceb6fcc478476a00f49ec3b36adfad))
* implement basic reference scaling ([f193203](https://github.com/cooklang/cookcli/commit/f193203ed65b26b4f58c2cde4d244135b8996d7e))
* import recipe ([27dccf4](https://github.com/cooklang/cookcli/commit/27dccf4c34c94e4743d2772652138b8ad8494bf9))
* new UI ([e200582](https://github.com/cooklang/cookcli/commit/e2005823bd51ef9eba87859e68b4af30b8501d4a))
* recursively get ingredients for referenced recipes ([bdc71cd](https://github.com/cooklang/cookcli/commit/bdc71cdc5df0b09651d953a724a165cb14844c4a))
* render scale ([83a90bb](https://github.com/cooklang/cookcli/commit/83a90bb429adc5c966aad82cbf5915ed7c499184))
* search via UI ([1ca4c2a](https://github.com/cooklang/cookcli/commit/1ca4c2ac543ebea29efd9dc0b40b3f8f4a40930f))
* support references in read command ([fc50c00](https://github.com/cooklang/cookcli/commit/fc50c0026cba26cd6866ce4587fd9762a9e65c03))
* support references in shopping list command ([9a959d4](https://github.com/cooklang/cookcli/commit/9a959d40a1c19f3b4f8b40ecce2ce7d1f5bfe049))
* support references in UI recipe screen ([50867de](https://github.com/cooklang/cookcli/commit/50867de9a99d26d2c98aa3b58cb01181ee5853af))
* support references in UI shopping list ([5090b83](https://github.com/cooklang/cookcli/commit/5090b83e0e2637092f056a52006acc29dc0becd0))
* support scaling ([66595f7](https://github.com/cooklang/cookcli/commit/66595f7559fa714378daaed31b7ee7e898bf5a4a))
* test release ([e86c34b](https://github.com/cooklang/cookcli/commit/e86c34be1e58b8e4fb1ae2e9a0fef20665610183))
* update package name ([fe5cf9c](https://github.com/cooklang/cookcli/commit/fe5cf9c21e0e582ae1d79b454a25bf7e650383b3))


### Bug Fixes

* brew install cmd ([adf9bce](https://github.com/cooklang/cookcli/commit/adf9bced58e1f102a18371a698cd0a03cebff0f7))
* cargo publish to include ui ([0807f55](https://github.com/cooklang/cookcli/commit/0807f55c633ac9932a81f5b32e32b6a6052061e8))
* ci build ([0b023a3](https://github.com/cooklang/cookcli/commit/0b023a3f001f7a6656ee2a7e054892cce7d49cdd))
* ci build ([5f62c85](https://github.com/cooklang/cookcli/commit/5f62c854b51526e6557af918b2e32c49395d99d4))
* cleanup unused code ([5672dca](https://github.com/cooklang/cookcli/commit/5672dcaf576ee2af6cf10f923baf6f1a193792a2))
* clippy ([edd9f98](https://github.com/cooklang/cookcli/commit/edd9f98ecab984cf159e6b07a76b8cded2999d10))
* issue with . base path in server ([85b91b0](https://github.com/cooklang/cookcli/commit/85b91b04537903b80e05e4f2cbc3a666029a3c5f))
* log to stdout using tracing subscriber. ([2e3ddab](https://github.com/cooklang/cookcli/commit/2e3ddab44680231f9dbbca462c9b15ea1c3947b7)), closes [#94](https://github.com/cooklang/cookcli/issues/94)
* make cost report work ([3538d20](https://github.com/cooklang/cookcli/commit/3538d2043489df9bd0994ee22f648edd1a3e3da1))
* shopping list ([4b54689](https://github.com/cooklang/cookcli/commit/4b54689d981ea2cb8adfa89ab8fe5515de1a2ee8))
* UI datastructure ([9265dcf](https://github.com/cooklang/cookcli/commit/9265dcf1159f12339d4a7cb905bec954008216df))
* upgraded parser ([d6c791f](https://github.com/cooklang/cookcli/commit/d6c791fd4c5ffd415c51fd03aa06d9a9478c36bd))
* use macos12 ([20a45d1](https://github.com/cooklang/cookcli/commit/20a45d108360de68ebe3efdbdfc2a5eef1c8226d))
* version ([84996f9](https://github.com/cooklang/cookcli/commit/84996f9b2fb3a54bc7d6868c32855ee4dec8b602))


### Miscellaneous Chores

* release 0.13.0 ([60b3ee5](https://github.com/cooklang/cookcli/commit/60b3ee5e9f1c4fabf95e5c7d15b1cc836632772d))
* release 0.15.1 ([579af82](https://github.com/cooklang/cookcli/commit/579af82b5633f61b5cdf4cbe40e325b24fbe8457))


### Code Refactoring

* use : as scaling factor delimiter ([6b2251c](https://github.com/cooklang/cookcli/commit/6b2251cc3ef16bb321d703658e2e67c99e0e9a33))

## [0.15.1](https://github.com/cooklang/cookcli/compare/v0.15.0...v0.15.1) (2025-08-28)


### ⚠ BREAKING CHANGES

* use : as scaling factor delimiter

### Features

* add base path for shopping lists so we can lookup references ([a5c1a42](https://github.com/cooklang/cookcli/commit/a5c1a42fd3dfb1c3ca63cfb6454ca47110789853))
* add basic search ([94741e0](https://github.com/cooklang/cookcli/commit/94741e0e712ebea7bc79db41021df768dc750075))
* add cache ([91ec988](https://github.com/cooklang/cookcli/commit/91ec988b5bcbdf089c061761ce25e19b53e853a8))
* add servings ([9f539d9](https://github.com/cooklang/cookcli/commit/9f539d9c2e901f53785ecf3edbd962b47003b276))
* add support for templates ([881495f](https://github.com/cooklang/cookcli/commit/881495f8003b21fe1084b209a9428ef231632179))
* cook doctor ([d46b166](https://github.com/cooklang/cookcli/commit/d46b16607ba687501e2808703b0a9910db83a3b6))
* correct repo name ([d6a4d0d](https://github.com/cooklang/cookcli/commit/d6a4d0d24c001a6fbc0e667e591e0919c11b3b58))
* detect cycle references ([3d9144f](https://github.com/cooklang/cookcli/commit/3d9144f66af5420440921a9e52883c9a8fc09b3c))
* diplay relative path for search results ([ac7f847](https://github.com/cooklang/cookcli/commit/ac7f847ef7dba190af678d47da736da0bebd707b))
* disable testing mode ([c46bf05](https://github.com/cooklang/cookcli/commit/c46bf05e1107f57f56f5490ff8ffd74c8c15f748))
* highlight sections in shopping list ([342db6c](https://github.com/cooklang/cookcli/commit/342db6c6b2ceb6fcc478476a00f49ec3b36adfad))
* implement basic reference scaling ([f193203](https://github.com/cooklang/cookcli/commit/f193203ed65b26b4f58c2cde4d244135b8996d7e))
* import recipe ([27dccf4](https://github.com/cooklang/cookcli/commit/27dccf4c34c94e4743d2772652138b8ad8494bf9))
* new UI ([e200582](https://github.com/cooklang/cookcli/commit/e2005823bd51ef9eba87859e68b4af30b8501d4a))
* recursively get ingredients for referenced recipes ([bdc71cd](https://github.com/cooklang/cookcli/commit/bdc71cdc5df0b09651d953a724a165cb14844c4a))
* render scale ([83a90bb](https://github.com/cooklang/cookcli/commit/83a90bb429adc5c966aad82cbf5915ed7c499184))
* search via UI ([1ca4c2a](https://github.com/cooklang/cookcli/commit/1ca4c2ac543ebea29efd9dc0b40b3f8f4a40930f))
* support references in read command ([fc50c00](https://github.com/cooklang/cookcli/commit/fc50c0026cba26cd6866ce4587fd9762a9e65c03))
* support references in shopping list command ([9a959d4](https://github.com/cooklang/cookcli/commit/9a959d40a1c19f3b4f8b40ecce2ce7d1f5bfe049))
* support references in UI recipe screen ([50867de](https://github.com/cooklang/cookcli/commit/50867de9a99d26d2c98aa3b58cb01181ee5853af))
* support references in UI shopping list ([5090b83](https://github.com/cooklang/cookcli/commit/5090b83e0e2637092f056a52006acc29dc0becd0))
* support scaling ([66595f7](https://github.com/cooklang/cookcli/commit/66595f7559fa714378daaed31b7ee7e898bf5a4a))
* test release ([e86c34b](https://github.com/cooklang/cookcli/commit/e86c34be1e58b8e4fb1ae2e9a0fef20665610183))
* update package name ([fe5cf9c](https://github.com/cooklang/cookcli/commit/fe5cf9c21e0e582ae1d79b454a25bf7e650383b3))


### Bug Fixes

* brew install cmd ([adf9bce](https://github.com/cooklang/cookcli/commit/adf9bced58e1f102a18371a698cd0a03cebff0f7))
* cargo publish to include ui ([0807f55](https://github.com/cooklang/cookcli/commit/0807f55c633ac9932a81f5b32e32b6a6052061e8))
* ci build ([5f62c85](https://github.com/cooklang/cookcli/commit/5f62c854b51526e6557af918b2e32c49395d99d4))
* cleanup unused code ([5672dca](https://github.com/cooklang/cookcli/commit/5672dcaf576ee2af6cf10f923baf6f1a193792a2))
* clippy ([edd9f98](https://github.com/cooklang/cookcli/commit/edd9f98ecab984cf159e6b07a76b8cded2999d10))
* issue with . base path in server ([85b91b0](https://github.com/cooklang/cookcli/commit/85b91b04537903b80e05e4f2cbc3a666029a3c5f))
* log to stdout using tracing subscriber. ([2e3ddab](https://github.com/cooklang/cookcli/commit/2e3ddab44680231f9dbbca462c9b15ea1c3947b7)), closes [#94](https://github.com/cooklang/cookcli/issues/94)
* make cost report work ([3538d20](https://github.com/cooklang/cookcli/commit/3538d2043489df9bd0994ee22f648edd1a3e3da1))
* shopping list ([4b54689](https://github.com/cooklang/cookcli/commit/4b54689d981ea2cb8adfa89ab8fe5515de1a2ee8))
* UI datastructure ([9265dcf](https://github.com/cooklang/cookcli/commit/9265dcf1159f12339d4a7cb905bec954008216df))
* upgraded parser ([d6c791f](https://github.com/cooklang/cookcli/commit/d6c791fd4c5ffd415c51fd03aa06d9a9478c36bd))
* use macos12 ([20a45d1](https://github.com/cooklang/cookcli/commit/20a45d108360de68ebe3efdbdfc2a5eef1c8226d))
* version ([84996f9](https://github.com/cooklang/cookcli/commit/84996f9b2fb3a54bc7d6868c32855ee4dec8b602))


### Miscellaneous Chores

* release 0.13.0 ([60b3ee5](https://github.com/cooklang/cookcli/commit/60b3ee5e9f1c4fabf95e5c7d15b1cc836632772d))
* release 0.15.1 ([579af82](https://github.com/cooklang/cookcli/commit/579af82b5633f61b5cdf4cbe40e325b24fbe8457))


### Code Refactoring

* use : as scaling factor delimiter ([6b2251c](https://github.com/cooklang/cookcli/commit/6b2251cc3ef16bb321d703658e2e67c99e0e9a33))

## [0.15.0](https://github.com/cooklang/cookcli/compare/v0.14.0...v0.15.0) (2025-08-28)


### Features

* cook doctor ([d46b166](https://github.com/cooklang/cookcli/commit/d46b16607ba687501e2808703b0a9910db83a3b6))
* new UI ([e200582](https://github.com/cooklang/cookcli/commit/e2005823bd51ef9eba87859e68b4af30b8501d4a))

## [Unreleased]

### Features

* **UI**: Complete rewrite of web UI from Svelte to server-side rendered Askama templates with Tailwind CSS
* **doctor**: Add comprehensive recipe validation command with syntax checking, reference validation, and CI-friendly exit codes  
* **shopping-list**: Enhanced shopping list with persistent storage, menu support, and improved aggregation
* **recipes**: Add support for .menu files for weekly meal planning
* **server**: Improved server with better static file handling and shopping list persistence
* **import**: Add metadata output support with multiple formats (JSON, YAML, frontmatter) and metadata-only extraction option

### Bug Fixes

* Fix recipe reference resolution and scaling in shopping lists
* Improve error handling and user feedback across all commands

### Documentation

* Add comprehensive documentation for all commands in docs/ directory
* Add CLAUDE.md for AI-assisted development guidance
* Update README with detailed command examples and usage patterns

### Dependencies

* Update cooklang parser to latest version for improved parsing reliability

### Infrastructure

* Add Dockerfile for containerized deployments
* Improve Makefile with CSS build targets for Tailwind compilation
* Add npm package.json for frontend build tooling

## [0.14.0](https://github.com/cooklang/cookcli/compare/v0.13.0...v0.14.0) (2025-06-03)


### Features

* add basic search ([94741e0](https://github.com/cooklang/cookcli/commit/94741e0e712ebea7bc79db41021df768dc750075))
* add support for templates ([881495f](https://github.com/cooklang/cookcli/commit/881495f8003b21fe1084b209a9428ef231632179))
* diplay relative path for search results ([ac7f847](https://github.com/cooklang/cookcli/commit/ac7f847ef7dba190af678d47da736da0bebd707b))
* import recipe ([27dccf4](https://github.com/cooklang/cookcli/commit/27dccf4c34c94e4743d2772652138b8ad8494bf9))
* search via UI ([1ca4c2a](https://github.com/cooklang/cookcli/commit/1ca4c2ac543ebea29efd9dc0b40b3f8f4a40930f))


### Bug Fixes

* make cost report work ([3538d20](https://github.com/cooklang/cookcli/commit/3538d2043489df9bd0994ee22f648edd1a3e3da1))

## [0.13.0](https://github.com/cooklang/cookcli/compare/v0.13.0...v0.13.0) (2025-05-27)


### ⚠ BREAKING CHANGES

* use : as scaling factor delimiter

### Features

* add base path for shopping lists so we can lookup references ([a5c1a42](https://github.com/cooklang/cookcli/commit/a5c1a42fd3dfb1c3ca63cfb6454ca47110789853))
* add cache ([91ec988](https://github.com/cooklang/cookcli/commit/91ec988b5bcbdf089c061761ce25e19b53e853a8))
* add servings ([9f539d9](https://github.com/cooklang/cookcli/commit/9f539d9c2e901f53785ecf3edbd962b47003b276))
* correct repo name ([d6a4d0d](https://github.com/cooklang/cookcli/commit/d6a4d0d24c001a6fbc0e667e591e0919c11b3b58))
* detect cycle references ([3d9144f](https://github.com/cooklang/cookcli/commit/3d9144f66af5420440921a9e52883c9a8fc09b3c))
* disable testing mode ([c46bf05](https://github.com/cooklang/cookcli/commit/c46bf05e1107f57f56f5490ff8ffd74c8c15f748))
* highlight sections in shopping list ([342db6c](https://github.com/cooklang/cookcli/commit/342db6c6b2ceb6fcc478476a00f49ec3b36adfad))
* implement basic reference scaling ([f193203](https://github.com/cooklang/cookcli/commit/f193203ed65b26b4f58c2cde4d244135b8996d7e))
* recursively get ingredients for referenced recipes ([bdc71cd](https://github.com/cooklang/cookcli/commit/bdc71cdc5df0b09651d953a724a165cb14844c4a))
* render scale ([83a90bb](https://github.com/cooklang/cookcli/commit/83a90bb429adc5c966aad82cbf5915ed7c499184))
* support references in read command ([fc50c00](https://github.com/cooklang/cookcli/commit/fc50c0026cba26cd6866ce4587fd9762a9e65c03))
* support references in shopping list command ([9a959d4](https://github.com/cooklang/cookcli/commit/9a959d40a1c19f3b4f8b40ecce2ce7d1f5bfe049))
* support references in UI recipe screen ([50867de](https://github.com/cooklang/cookcli/commit/50867de9a99d26d2c98aa3b58cb01181ee5853af))
* support references in UI shopping list ([5090b83](https://github.com/cooklang/cookcli/commit/5090b83e0e2637092f056a52006acc29dc0becd0))
* support scaling ([66595f7](https://github.com/cooklang/cookcli/commit/66595f7559fa714378daaed31b7ee7e898bf5a4a))
* test release ([e86c34b](https://github.com/cooklang/cookcli/commit/e86c34be1e58b8e4fb1ae2e9a0fef20665610183))
* update package name ([fe5cf9c](https://github.com/cooklang/cookcli/commit/fe5cf9c21e0e582ae1d79b454a25bf7e650383b3))


### Bug Fixes

* brew install cmd ([adf9bce](https://github.com/cooklang/cookcli/commit/adf9bced58e1f102a18371a698cd0a03cebff0f7))
* cargo publish to include ui ([0807f55](https://github.com/cooklang/cookcli/commit/0807f55c633ac9932a81f5b32e32b6a6052061e8))
* cleanup unused code ([5672dca](https://github.com/cooklang/cookcli/commit/5672dcaf576ee2af6cf10f923baf6f1a193792a2))
* clippy ([edd9f98](https://github.com/cooklang/cookcli/commit/edd9f98ecab984cf159e6b07a76b8cded2999d10))
* issue with . base path in server ([85b91b0](https://github.com/cooklang/cookcli/commit/85b91b04537903b80e05e4f2cbc3a666029a3c5f))
* log to stdout using tracing subscriber. ([2e3ddab](https://github.com/cooklang/cookcli/commit/2e3ddab44680231f9dbbca462c9b15ea1c3947b7)), closes [#94](https://github.com/cooklang/cookcli/issues/94)
* shopping list ([4b54689](https://github.com/cooklang/cookcli/commit/4b54689d981ea2cb8adfa89ab8fe5515de1a2ee8))
* UI datastructure ([9265dcf](https://github.com/cooklang/cookcli/commit/9265dcf1159f12339d4a7cb905bec954008216df))
* upgraded parser ([d6c791f](https://github.com/cooklang/cookcli/commit/d6c791fd4c5ffd415c51fd03aa06d9a9478c36bd))
* use macos12 ([20a45d1](https://github.com/cooklang/cookcli/commit/20a45d108360de68ebe3efdbdfc2a5eef1c8226d))
* version ([84996f9](https://github.com/cooklang/cookcli/commit/84996f9b2fb3a54bc7d6868c32855ee4dec8b602))


### Miscellaneous Chores

* release 0.13.0 ([60b3ee5](https://github.com/cooklang/cookcli/commit/60b3ee5e9f1c4fabf95e5c7d15b1cc836632772d))


### Code Refactoring

* use : as scaling factor delimiter ([6b2251c](https://github.com/cooklang/cookcli/commit/6b2251cc3ef16bb321d703658e2e67c99e0e9a33))

## [0.13.0](https://github.com/cooklang/cookcli/compare/v0.12.1...v0.13.0) (2025-05-27)


### ⚠ BREAKING CHANGES

* use : as scaling factor delimiter

### Features

* add base path for shopping lists so we can lookup references ([a5c1a42](https://github.com/cooklang/cookcli/commit/a5c1a42fd3dfb1c3ca63cfb6454ca47110789853))
* detect cycle references ([3d9144f](https://github.com/cooklang/cookcli/commit/3d9144f66af5420440921a9e52883c9a8fc09b3c))
* highlight sections in shopping list ([342db6c](https://github.com/cooklang/cookcli/commit/342db6c6b2ceb6fcc478476a00f49ec3b36adfad))
* implement basic reference scaling ([f193203](https://github.com/cooklang/cookcli/commit/f193203ed65b26b4f58c2cde4d244135b8996d7e))
* recursively get ingredients for referenced recipes ([bdc71cd](https://github.com/cooklang/cookcli/commit/bdc71cdc5df0b09651d953a724a165cb14844c4a))
* support references in read command ([fc50c00](https://github.com/cooklang/cookcli/commit/fc50c0026cba26cd6866ce4587fd9762a9e65c03))
* support references in shopping list command ([9a959d4](https://github.com/cooklang/cookcli/commit/9a959d40a1c19f3b4f8b40ecce2ce7d1f5bfe049))
* support references in UI recipe screen ([50867de](https://github.com/cooklang/cookcli/commit/50867de9a99d26d2c98aa3b58cb01181ee5853af))
* support references in UI shopping list ([5090b83](https://github.com/cooklang/cookcli/commit/5090b83e0e2637092f056a52006acc29dc0becd0))


### Bug Fixes

* brew install cmd ([adf9bce](https://github.com/cooklang/cookcli/commit/adf9bced58e1f102a18371a698cd0a03cebff0f7))
* cargo publish to include ui ([0807f55](https://github.com/cooklang/cookcli/commit/0807f55c633ac9932a81f5b32e32b6a6052061e8))


### Miscellaneous Chores

* release 0.13.0 ([60b3ee5](https://github.com/cooklang/cookcli/commit/60b3ee5e9f1c4fabf95e5c7d15b1cc836632772d))


### Code Refactoring

* use : as scaling factor delimiter ([6b2251c](https://github.com/cooklang/cookcli/commit/6b2251cc3ef16bb321d703658e2e67c99e0e9a33))

## [0.12.1](https://github.com/cooklang/cookcli/compare/v0.12.0...v0.12.1) (2025-05-22)


### Bug Fixes

* issue with . base path in server ([85b91b0](https://github.com/cooklang/cookcli/commit/85b91b04537903b80e05e4f2cbc3a666029a3c5f))

## [0.12.0](https://github.com/cooklang/cookcli/compare/v0.11.0...v0.12.0) (2025-05-21)


### Features

* add cache ([91ec988](https://github.com/cooklang/cookcli/commit/91ec988b5bcbdf089c061761ce25e19b53e853a8))
* add servings ([9f539d9](https://github.com/cooklang/cookcli/commit/9f539d9c2e901f53785ecf3edbd962b47003b276))
* correct repo name ([d6a4d0d](https://github.com/cooklang/cookcli/commit/d6a4d0d24c001a6fbc0e667e591e0919c11b3b58))
* disable testing mode ([c46bf05](https://github.com/cooklang/cookcli/commit/c46bf05e1107f57f56f5490ff8ffd74c8c15f748))
* render scale ([83a90bb](https://github.com/cooklang/cookcli/commit/83a90bb429adc5c966aad82cbf5915ed7c499184))
* support scaling ([66595f7](https://github.com/cooklang/cookcli/commit/66595f7559fa714378daaed31b7ee7e898bf5a4a))
* test release ([e86c34b](https://github.com/cooklang/cookcli/commit/e86c34be1e58b8e4fb1ae2e9a0fef20665610183))
* update package name ([fe5cf9c](https://github.com/cooklang/cookcli/commit/fe5cf9c21e0e582ae1d79b454a25bf7e650383b3))


### Bug Fixes

* cleanup unused code ([5672dca](https://github.com/cooklang/cookcli/commit/5672dcaf576ee2af6cf10f923baf6f1a193792a2))
* clippy ([edd9f98](https://github.com/cooklang/cookcli/commit/edd9f98ecab984cf159e6b07a76b8cded2999d10))
* log to stdout using tracing subscriber. ([2e3ddab](https://github.com/cooklang/cookcli/commit/2e3ddab44680231f9dbbca462c9b15ea1c3947b7)), closes [#94](https://github.com/cooklang/cookcli/issues/94)
* shopping list ([4b54689](https://github.com/cooklang/cookcli/commit/4b54689d981ea2cb8adfa89ab8fe5515de1a2ee8))
* UI datastructure ([9265dcf](https://github.com/cooklang/cookcli/commit/9265dcf1159f12339d4a7cb905bec954008216df))
* upgraded parser ([d6c791f](https://github.com/cooklang/cookcli/commit/d6c791fd4c5ffd415c51fd03aa06d9a9478c36bd))
* use macos12 ([20a45d1](https://github.com/cooklang/cookcli/commit/20a45d108360de68ebe3efdbdfc2a5eef1c8226d))
* version ([84996f9](https://github.com/cooklang/cookcli/commit/84996f9b2fb3a54bc7d6868c32855ee4dec8b602))

## [0.11.0](https://github.com/cooklang/cookcli/compare/v0.10.0...v0.11.0) (2025-05-21)


### Features

* add servings ([9f539d9](https://github.com/cooklang/cookcli/commit/9f539d9c2e901f53785ecf3edbd962b47003b276))
* render scale ([83a90bb](https://github.com/cooklang/cookcli/commit/83a90bb429adc5c966aad82cbf5915ed7c499184))
* support scaling ([66595f7](https://github.com/cooklang/cookcli/commit/66595f7559fa714378daaed31b7ee7e898bf5a4a))
* update package name ([fe5cf9c](https://github.com/cooklang/cookcli/commit/fe5cf9c21e0e582ae1d79b454a25bf7e650383b3))


### Bug Fixes

* cleanup unused code ([5672dca](https://github.com/cooklang/cookcli/commit/5672dcaf576ee2af6cf10f923baf6f1a193792a2))
* clippy ([edd9f98](https://github.com/cooklang/cookcli/commit/edd9f98ecab984cf159e6b07a76b8cded2999d10))
* UI datastructure ([9265dcf](https://github.com/cooklang/cookcli/commit/9265dcf1159f12339d4a7cb905bec954008216df))

## [0.10.0](https://github.com/cooklang/cookcli/compare/v0.9.0...v0.10.0) (2025-01-26)


### Features

* add cache ([91ec988](https://github.com/cooklang/cookcli/commit/91ec988b5bcbdf089c061761ce25e19b53e853a8))
* correct repo name ([d6a4d0d](https://github.com/cooklang/cookcli/commit/d6a4d0d24c001a6fbc0e667e591e0919c11b3b58))
* disable testing mode ([c46bf05](https://github.com/cooklang/cookcli/commit/c46bf05e1107f57f56f5490ff8ffd74c8c15f748))
* test release ([e86c34b](https://github.com/cooklang/cookcli/commit/e86c34be1e58b8e4fb1ae2e9a0fef20665610183))


### Bug Fixes

* log to stdout using tracing subscriber. ([2e3ddab](https://github.com/cooklang/cookcli/commit/2e3ddab44680231f9dbbca462c9b15ea1c3947b7)), closes [#94](https://github.com/cooklang/cookcli/issues/94)
* shopping list ([4b54689](https://github.com/cooklang/cookcli/commit/4b54689d981ea2cb8adfa89ab8fe5515de1a2ee8))
* upgraded parser ([d6c791f](https://github.com/cooklang/cookcli/commit/d6c791fd4c5ffd415c51fd03aa06d9a9478c36bd))
* use macos12 ([20a45d1](https://github.com/cooklang/cookcli/commit/20a45d108360de68ebe3efdbdfc2a5eef1c8226d))
* version ([84996f9](https://github.com/cooklang/cookcli/commit/84996f9b2fb3a54bc7d6868c32855ee4dec8b602))

## [0.9.0](https://github.com/cooklang/cookcli/compare/v0.8.1...v0.9.0) (2025-01-26)


### Features

* add cache ([91ec988](https://github.com/cooklang/cookcli/commit/91ec988b5bcbdf089c061761ce25e19b53e853a8))
* correct repo name ([d6a4d0d](https://github.com/cooklang/cookcli/commit/d6a4d0d24c001a6fbc0e667e591e0919c11b3b58))
* disable testing mode ([c46bf05](https://github.com/cooklang/cookcli/commit/c46bf05e1107f57f56f5490ff8ffd74c8c15f748))
* test release ([e86c34b](https://github.com/cooklang/cookcli/commit/e86c34be1e58b8e4fb1ae2e9a0fef20665610183))


### Bug Fixes

* log to stdout using tracing subscriber. ([2e3ddab](https://github.com/cooklang/cookcli/commit/2e3ddab44680231f9dbbca462c9b15ea1c3947b7)), closes [#94](https://github.com/cooklang/cookcli/issues/94)
* shopping list ([4b54689](https://github.com/cooklang/cookcli/commit/4b54689d981ea2cb8adfa89ab8fe5515de1a2ee8))
* upgraded parser ([d6c791f](https://github.com/cooklang/cookcli/commit/d6c791fd4c5ffd415c51fd03aa06d9a9478c36bd))
* use macos12 ([20a45d1](https://github.com/cooklang/cookcli/commit/20a45d108360de68ebe3efdbdfc2a5eef1c8226d))
* version ([84996f9](https://github.com/cooklang/cookcli/commit/84996f9b2fb3a54bc7d6868c32855ee4dec8b602))

## [0.8.1](https://github.com/cooklang/cookcli/compare/v0.8.0...v0.8.1) (2025-01-26)


### Bug Fixes

* shopping list ([4b54689](https://github.com/cooklang/cookcli/commit/4b54689d981ea2cb8adfa89ab8fe5515de1a2ee8))
* upgraded parser ([d6c791f](https://github.com/cooklang/cookcli/commit/d6c791fd4c5ffd415c51fd03aa06d9a9478c36bd))
* version ([84996f9](https://github.com/cooklang/cookcli/commit/84996f9b2fb3a54bc7d6868c32855ee4dec8b602))

## [0.8.0](https://github.com/cooklang/cookcli/compare/v0.7.1...v0.8.0) (2024-01-14)


### Features

* add cache ([91ec988](https://github.com/cooklang/cookcli/commit/91ec988b5bcbdf089c061761ce25e19b53e853a8))
* correct repo name ([d6a4d0d](https://github.com/cooklang/cookcli/commit/d6a4d0d24c001a6fbc0e667e591e0919c11b3b58))
* disable testing mode ([c46bf05](https://github.com/cooklang/cookcli/commit/c46bf05e1107f57f56f5490ff8ffd74c8c15f748))
* test release ([e86c34b](https://github.com/cooklang/cookcli/commit/e86c34be1e58b8e4fb1ae2e9a0fef20665610183))


### Bug Fixes

* log to stdout using tracing subscriber. ([2e3ddab](https://github.com/cooklang/cookcli/commit/2e3ddab44680231f9dbbca462c9b15ea1c3947b7)), closes [#94](https://github.com/cooklang/cookcli/issues/94)

## [0.7.1](https://github.com/cooklang/cookcli/compare/v0.7.0...v0.7.1) (2024-01-13)


### Bug Fixes

* log to stdout using tracing subscriber. ([2e3ddab](https://github.com/cooklang/cookcli/commit/2e3ddab44680231f9dbbca462c9b15ea1c3947b7)), closes [#94](https://github.com/cooklang/cookcli/issues/94)

## [0.7.0](https://github.com/cooklang/cookcli/compare/v0.6.0...v0.7.0) (2023-12-11)


### Features

* add cache ([91ec988](https://github.com/cooklang/cookcli/commit/91ec988b5bcbdf089c061761ce25e19b53e853a8))
* correct repo name ([d6a4d0d](https://github.com/cooklang/cookcli/commit/d6a4d0d24c001a6fbc0e667e591e0919c11b3b58))
* disable testing mode ([c46bf05](https://github.com/cooklang/cookcli/commit/c46bf05e1107f57f56f5490ff8ffd74c8c15f748))
* test release ([e86c34b](https://github.com/cooklang/cookcli/commit/e86c34be1e58b8e4fb1ae2e9a0fef20665610183))

## [0.6.0](https://github.com/cooklang/cookcli/compare/v0.5.0...v0.6.0) (2023-12-11)


### Features

* add cache ([91ec988](https://github.com/cooklang/cookcli/commit/91ec988b5bcbdf089c061761ce25e19b53e853a8))
* disable testing mode ([c46bf05](https://github.com/cooklang/cookcli/commit/c46bf05e1107f57f56f5490ff8ffd74c8c15f748))

## [0.5.0](https://github.com/cooklang/CookCLI/compare/v0.4.0...v0.5.0) (2023-11-22)


### Features

* correct repo name ([d6a4d0d](https://github.com/cooklang/CookCLI/commit/d6a4d0d24c001a6fbc0e667e591e0919c11b3b58))
* test release ([e86c34b](https://github.com/cooklang/CookCLI/commit/e86c34be1e58b8e4fb1ae2e9a0fef20665610183))

## [0.4.0](https://github.com/cooklang/CookCLI/compare/v0.3.0...v0.4.0) (2023-11-22)


### Features

* correct repo name ([d6a4d0d](https://github.com/cooklang/CookCLI/commit/d6a4d0d24c001a6fbc0e667e591e0919c11b3b58))
* test release ([e86c34b](https://github.com/cooklang/CookCLI/commit/e86c34be1e58b8e4fb1ae2e9a0fef20665610183))

## [0.3.0](https://github.com/cooklang/CookCLI/compare/v0.2.1...v0.3.0) (2023-11-22)


### Features

* test release ([e86c34b](https://github.com/cooklang/CookCLI/commit/e86c34be1e58b8e4fb1ae2e9a0fef20665610183))
