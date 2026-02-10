# Changelog

## [0.22.0](https://github.com/cooklang/cookcli/compare/v0.21.0...v0.22.0) (2026-02-10)


### Features

* add keyboard shortcuts for web UI ([cb1d99d](https://github.com/cooklang/cookcli/commit/cb1d99dc83d1102026af826c2121c912bfb14a05)), closes [#248](https://github.com/cooklang/cookcli/issues/248)
* support backslash line breaks for formatting in steps and notes ([40f4987](https://github.com/cooklang/cookcli/commit/40f4987753a07f5d3e8fa8886974d357f00f2625))


### Bug Fixes

* allow dirty working directory for cargo publish and format code ([36b0ec0](https://github.com/cooklang/cookcli/commit/36b0ec01ca300eb09bfa72847763992c2539d571))
* issues with line break ([5edc0ec](https://github.com/cooklang/cookcli/commit/5edc0ec2a93091aa5fb34de4d233e3a19b0feae2))
* optimize web UI layout for tablet-sized screens ([2e1d1b7](https://github.com/cooklang/cookcli/commit/2e1d1b7005521ac28ef0c1fb72960dfa56a57547))
* resolve clippy warnings for unnecessary_unwrap ([d38e669](https://github.com/cooklang/cookcli/commit/d38e669401d2ae0571d4484eea41c45eb6bae929))
* update dependencies to resolve security vulnerabilities ([52ec667](https://github.com/cooklang/cookcli/commit/52ec667f167acdb916216d2ff418c80e39c2baae))
* use if-let instead of is_some + unwrap for ingredient references ([22440a6](https://github.com/cooklang/cookcli/commit/22440a6f95a29ae6f16261da72e22ad2e3c9fdd7))

## [0.21.0](https://github.com/cooklang/cookcli/compare/v0.20.0...v0.21.0) (2026-01-21)


### Features

* **api:** add GET /api/recipes/raw/{path} endpoint ([423c0c6](https://github.com/cooklang/cookcli/commit/423c0c66d21205b59cfecbfd02120310896f0417))
* **api:** add PUT /api/recipes/{path} endpoint ([59da892](https://github.com/cooklang/cookcli/commit/59da892e4c1df24376ca0c0470c5378edf1866d1))
* **editor:** add autosave, remove preview, rename Cancel to Back ([0fef824](https://github.com/cooklang/cookcli/commit/0fef824746e2065cb01f7c552fbbf0e402b528a9))
* **editor:** add CodeMirror editor entry point ([a22bf4a](https://github.com/cooklang/cookcli/commit/a22bf4aeca43f0a46afa916a03e32889b23c4f9c))
* **editor:** add Cooklang syntax mode for CodeMirror 6 ([42bcebb](https://github.com/cooklang/cookcli/commit/42bcebbc04404456263f6fb3ee90c92ef0c315f7))
* **editor:** add delete recipe, improve editor UX ([f64365b](https://github.com/cooklang/cookcli/commit/f64365bb5efad413b980e48770687caff97cc212))
* **editor:** add LSP connection status indicator ([72d007a](https://github.com/cooklang/cookcli/commit/72d007acb9460f680cf363c58727a71b8d9e3dd6))
* **editor:** add LSP document synchronization ([14baf81](https://github.com/cooklang/cookcli/commit/14baf81850808d4cb2d4628d4dfad7ce39426746))
* **editor:** add LSP-powered autocomplete ([28b95aa](https://github.com/cooklang/cookcli/commit/28b95aa7d1b9feb13c23059e69e7dcac3906dcc1))
* **editor:** add new recipe creation page ([20fd5f4](https://github.com/cooklang/cookcli/commit/20fd5f4d726725c907461a9eb662bd29f4ef6dca))
* **editor:** add preview toggle ([1f40bb6](https://github.com/cooklang/cookcli/commit/1f40bb677c64d57d89111bb3f83ea39b51ad4882))
* **editor:** display LSP diagnostics in editor ([30e34b9](https://github.com/cooklang/cookcli/commit/30e34b93e00cb8ec310330e371741811452a7fb2))
* **editor:** integrate CodeMirror with syntax highlighting ([3d8849e](https://github.com/cooklang/cookcli/commit/3d8849e2bdbe71e3fc2de385f59315df97bc3454))
* **editor:** Phase 4 - LSP client & full features ([8e5f98f](https://github.com/cooklang/cookcli/commit/8e5f98f38b7b6416c6eab55fd141c2f3906c4043))
* **i18n:** add editor translation keys ([abb341c](https://github.com/cooklang/cookcli/commit/abb341c8051235f806e57cb21dd0b3209cba24c1))
* **i18n:** add LSP status translation keys ([39ce0fa](https://github.com/cooklang/cookcli/commit/39ce0faf670227b600ea43060660147de0673dfc))
* improve PWA [#239](https://github.com/cooklang/cookcli/issues/239) ([aacebac](https://github.com/cooklang/cookcli/commit/aacebac355758fa4e5e2b969a78d8875091b2f64))
* **lsp:** add WebSocket to LSP subprocess bridge ([7e0d6d7](https://github.com/cooklang/cookcli/commit/7e0d6d71ccfd3bbd9006ab63f2d0d554d0ad9d7c))
* **lsp:** register WebSocket LSP endpoint at /api/ws/lsp ([a02251d](https://github.com/cooklang/cookcli/commit/a02251d59367cb9ed3f7aca96451a122a977b4b2))
* **ui:** add /edit/{path} route for recipe editor ([d7a30b9](https://github.com/cooklang/cookcli/commit/d7a30b9bd5cd6e7f5074443cef9aaaf3f74e022f))
* **ui:** add Edit button to menu page ([fd669e4](https://github.com/cooklang/cookcli/commit/fd669e4793cad1ee3de866c2459720e06ea385ab))
* **ui:** add Edit button to recipe detail page ([c84896f](https://github.com/cooklang/cookcli/commit/c84896f673fbd336b71d8bdeb061ebe91c9089af))
* **ui:** add editor template with textarea ([405cae3](https://github.com/cooklang/cookcli/commit/405cae34a3e485f5b3ca5a1dbcad099bf125f7aa))
* **ui:** add New Recipe button to recipes list ([926e069](https://github.com/cooklang/cookcli/commit/926e069d348942290aaede43904d6b22cdf66c55))


### Bug Fixes

* add 1MB request body size limit ([8ffd54a](https://github.com/cooklang/cookcli/commit/8ffd54a5ef3e57df5b1e89f7ae7f0c2baa9643b9))
* address TOCTOU race condition in create_recipe ([ad14fa8](https://github.com/cooklang/cookcli/commit/ad14fa8cf892da88a25033c5c17decbce22f5e35))
* clippy ([252905c](https://github.com/cooklang/cookcli/commit/252905cabd6f7e5acd431a1d6c4607aa9f6dd836))
* display errors to user via toast notifications ([ae6d1da](https://github.com/cooklang/cookcli/commit/ae6d1da379d167801462f5ddf612abbf94985ea8))
* **editor:** address code review feedback for recipe creation ([7343753](https://github.com/cooklang/cookcli/commit/7343753b82c0b7af17f80d49ce2586e72602eea1))
* **editor:** pass base_path to LSP for aisle.conf loading ([35dfc04](https://github.com/cooklang/cookcli/commit/35dfc045f415c9e2b0832ca19d3650072fe1e85d))
* **editor:** prevent HTML escaping of JSON in template ([af31eb9](https://github.com/cooklang/cookcli/commit/af31eb9be833e9eb0a095a891cddbab7f0ead3fb))
* **editor:** reset line-scoped states and fix Makefile clean target ([20a3063](https://github.com/cooklang/cookcli/commit/20a3063cce388a182d05325978d9bfb7cb97d3d6))
* **editor:** use HighlightStyle for proper syntax highlighting ([cce9148](https://github.com/cooklang/cookcli/commit/cce914891b21887afd3cd2ffae2599956f7426b6))
* fmt ([0f0e48e](https://github.com/cooklang/cookcli/commit/0f0e48eb4ca58dc1f4b3ca6abea7e1615fb2602b))
* fmt ([f919af2](https://github.com/cooklang/cookcli/commit/f919af2b5a58e3cab0695a1e28b26e17a6a37d94))
* improve LSP bridge task cleanup and document buffer size ([88dadc0](https://github.com/cooklang/cookcli/commit/88dadc0c516b9cc783b924891890078891537906))
* lsp version ([c9447dc](https://github.com/cooklang/cookcli/commit/c9447dcc8a2101a908a92244821ebd6bb6d3dc2e))
* use published language srver ([197e528](https://github.com/cooklang/cookcli/commit/197e5281bb8eb4a98eb4a84882e65e41d47eeed4))
* use spawn_blocking for synchronous canonicalize calls ([8505956](https://github.com/cooklang/cookcli/commit/85059563c160e9d64674fdda1f0bd9432dace191))
* use tokio::fs for async file deletion in recipe_delete ([5e2b90c](https://github.com/cooklang/cookcli/commit/5e2b90c877fc78fcb503c56ba7b108336612c8ba))
* use tokio::fs for async file operations in recipe_save ([8be79a0](https://github.com/cooklang/cookcli/commit/8be79a059384ce6748964ef248068dfd8e84bf1b))
* use tokio::fs for async file reading in edit_page ([37a31f9](https://github.com/cooklang/cookcli/commit/37a31f97c27794ceee6d1a8ca54a8888e9e4401e))
* use tokio::fs for async file reading in pantry_page ([521575f](https://github.com/cooklang/cookcli/commit/521575f9cf0e88da7dd1a561de11710633e09b9c))
* use tokio::fs for async file reading in recipe_raw ([d2ce797](https://github.com/cooklang/cookcli/commit/d2ce79714855f73a402302f9bd057f09fdd399bc))
* validate filename before sanitization in create_recipe ([e0f4bae](https://github.com/cooklang/cookcli/commit/e0f4bae1d9459dcdac8cc1c6da587111b33bfa90))

## [0.20.0](https://github.com/cooklang/cookcli/compare/v0.19.3...v0.20.0) (2026-01-16)


### Features

* add cook lsp subcommand for language server support ([2d12e09](https://github.com/cooklang/cookcli/commit/2d12e09fdafd53e98abe958efa51799ad6873ae3))
* also publish in crate ([e42ed51](https://github.com/cooklang/cookcli/commit/e42ed5169488a5c21b10336bf0020c6944c10127))
* combine duplicate ingredients in recipe display ([debdbf0](https://github.com/cooklang/cookcli/commit/debdbf06de58e8c8723a8c249fe1d466ecd946bc))
* combine duplicate ingredients in web UI recipe display ([5f57b18](https://github.com/cooklang/cookcli/commit/5f57b181aa6826ae7a71b696d633d7897ff03050))
* log context paths in LSP for debugging ([8d372cd](https://github.com/cooklang/cookcli/commit/8d372cd9a29adf9b215ea0da085e6604946d1ca9))


### Bug Fixes

* display more context for errors ([5b0126a](https://github.com/cooklang/cookcli/commit/5b0126ab98d05bc377c2023d50132757c6e4d73b))
* extra space removed [#226](https://github.com/cooklang/cookcli/issues/226) ([4723b2d](https://github.com/cooklang/cookcli/commit/4723b2df40f9655a6b2bcfd4ecff9bb936ed45f3))

## [0.19.3](https://github.com/cooklang/cookcli/compare/v0.19.2...v0.19.3) (2026-01-04)


### Bug Fixes

* respect explicit low thresholds in pantry depleted command ([f2f8371](https://github.com/cooklang/cookcli/commit/f2f83717fe3d4533777785d78764f78659ed0f6f)), closes [#228](https://github.com/cooklang/cookcli/issues/228)
* update macOS runners from deprecated macOS-13 ([94cf605](https://github.com/cooklang/cookcli/commit/94cf60576a8967153d69706a963ddafca26f4fe8))

## [0.19.2](https://github.com/cooklang/cookcli/compare/v0.19.1...v0.19.2) (2025-12-28)


### Bug Fixes

* add Windows-specific snapshot for doctor validate test ([e543dbd](https://github.com/cooklang/cookcli/commit/e543dbd1c2c4fb87470f90bf9a1bac480f6c371a))
* use consistent path separator in doctor validate output ([f42d359](https://github.com/cooklang/cookcli/commit/f42d359fcf07422419d894270594f859f715bbc0))

## [0.19.1](https://github.com/cooklang/cookcli/compare/v0.19.0...v0.19.1) (2025-12-27)


### Features

* improve breadcrumb ([b9618a8](https://github.com/cooklang/cookcli/commit/b9618a855db56a9d114fb1ed6569cb43b0853076))
* remove ".cook" in the breadcrumb ([0cd458c](https://github.com/cooklang/cookcli/commit/0cd458c54ef285620993a5703d8223a166a98208))


### Bug Fixes

* add export of shopping list in markdown format ([11cbaa1](https://github.com/cooklang/cookcli/commit/11cbaa1740b93f91bc46b713be2cfd37cba58bbe))
* add export of shopping list in markdown format ([8a5e65c](https://github.com/cooklang/cookcli/commit/8a5e65cbae54b49706b1a481568841beab4e1c11))
* better alignment in the breadcrumb ([f148fdf](https://github.com/cooklang/cookcli/commit/f148fdfc25fbf2bb670b70611c7cbc8a94908320))
* notes are rendered in correct order between steps (web interface) ([e4ad193](https://github.com/cooklang/cookcli/commit/e4ad1933e426aa0df25b563d3bf3f25119ad85db))
* show pantry inventory even if pantry.conf is empty, but show link to preferences page only if pantry.conf is not found ([f535295](https://github.com/cooklang/cookcli/commit/f535295e93e8a96e36df442a095d8a97d306e9ea))
* use ingredient list with common names from aisle configuration ([61152c3](https://github.com/cooklang/cookcli/commit/61152c3a02f043f4fe9f16ff55e4527f7628fb93))


### Miscellaneous Chores

* release 0.19.1 ([4ded7b1](https://github.com/cooklang/cookcli/commit/4ded7b1a2fe5a2ea1a5e8666bbf0522339f7b620))

## [0.19.0](https://github.com/cooklang/cookcli/compare/v0.18.2...v0.19.0) (2025-11-26)


### Features

* add --allow-missing parameter for flexible pantry planning ([c30ffed](https://github.com/cooklang/cookcli/commit/c30ffedab452780d71b92bb45310fba8bafdf465))
* add --skip parameter to pantry plan command ([7441f8a](https://github.com/cooklang/cookcli/commit/7441f8a20142f197b21f7777193e6000e662245d))
* add pantry plan command to analyze ingredient usage ([04ed7b8](https://github.com/cooklang/cookcli/commit/04ed7b8960751a6a8c79ac9057a59d1388f346f0))
* added Cooking instructions ([4cbb414](https://github.com/cooklang/cookcli/commit/4cbb41414e6d587942ed2058bb6f2d35bf9f5e5b))
* added cookware ([f7490f3](https://github.com/cooklang/cookcli/commit/f7490f3b607fb79b8aa643313f5901be3d29c7a4))
* added metadata ([a836cc9](https://github.com/cooklang/cookcli/commit/a836cc9a3bbdb1477ab739e393e1e0b287118877))
* added tags ([d5121af](https://github.com/cooklang/cookcli/commit/d5121afebd90dcee951887e1b6b9b4fde86e2ec0))
* adjusted character escaping to Typst. This is a best effort since there is no official list. ([5825864](https://github.com/cooklang/cookcli/commit/582586428bb748210de83438c5718a69be79fee5))
* implemented simple header, footer and title but Style is not quite correct yet ([a6f4fe0](https://github.com/cooklang/cookcli/commit/a6f4fe0bfb7e3b320e1ad91b3183360e3ca285b7))
* ingredient list added. No multicol for now... ([01f5259](https://github.com/cooklang/cookcli/commit/01f525996c39157719435fe78a4eb9ea9cec2bc0))
* initial Hello World for Typst export ([f04790a](https://github.com/cooklang/cookcli/commit/f04790a43d75fbf1899596fbdbd039b55f7c565c))
* small adjustments ([02377a6](https://github.com/cooklang/cookcli/commit/02377a6de811998351527c86ba2b923b1f6885ff))


### Bug Fixes

* clean research ([8fddfd2](https://github.com/cooklang/cookcli/commit/8fddfd2f41f6ed53cc92a791f9c1f78dc1074107))
* minor cargo fmt changes ([eace0de](https://github.com/cooklang/cookcli/commit/eace0debf85f0bb883ed1819261ccd981ffae201))
* missing typst format ([d71cf71](https://github.com/cooklang/cookcli/commit/d71cf719f419207f2a2210b07b88ea3a1003ae5b))
* **server:** use filename for recipe path resolution ([3fba010](https://github.com/cooklang/cookcli/commit/3fba0102510d600f970758b9d62c4138d1b7d17f))
* **server:** use filename for recipe path resolution ([b22fc87](https://github.com/cooklang/cookcli/commit/b22fc87728586e7b5c309b525450de9bbb3f4332))
* Typst file extension ([94b1d2c](https://github.com/cooklang/cookcli/commit/94b1d2cba3e65c8600a4d82e1155b0641bf69b56))

## [0.18.2](https://github.com/cooklang/cookcli/compare/v0.18.1...v0.18.2) (2025-10-18)


### Bug Fixes

* always use base path for finding refsi ([338fbc7](https://github.com/cooklang/cookcli/commit/338fbc7abae99d1884d9473e4e7dae25144f4ce1))
* clippy ([ee687b9](https://github.com/cooklang/cookcli/commit/ee687b9810ece48e9bda21dbd8df29d0575f5b49))
* fmt ([2143f4a](https://github.com/cooklang/cookcli/commit/2143f4af21abaa915636c631c4f040300214a01e))
* proper separator ([1fd3f61](https://github.com/cooklang/cookcli/commit/1fd3f61bbf87834de9c35225b1072023e946b8ea))
* remove .claude ([2f21d24](https://github.com/cooklang/cookcli/commit/2f21d24aba457e4c460baab68500e7b11a8e7542))
* remove junk ([9c43d49](https://github.com/cooklang/cookcli/commit/9c43d49a4f4fe9aac01dceb64f4b69c0e87d25b0))
* return favicon https://github.com/cooklang/cookcli/issues/174 ([b807434](https://github.com/cooklang/cookcli/commit/b807434d11198e8ab4bc1986c966d6dac91d7184))
* review suggestions ([43add8b](https://github.com/cooklang/cookcli/commit/43add8b1b5769af7f205f7610cc4309e69457599))
* shorthand ingredients display ([1d36107](https://github.com/cooklang/cookcli/commit/1d3610753dd3d8a5b5696140fd84a775a6cf275c))
* shorthand ingredients display ([43287c7](https://github.com/cooklang/cookcli/commit/43287c7a6d0242ec0ccbefc4895c2345157d2748))
* spelling https://github.com/cooklang/cookcli/issues/123 ([b6d8d27](https://github.com/cooklang/cookcli/commit/b6d8d2731d917aa38404476f490fbd9b2a100a77))
* test ([db79534](https://github.com/cooklang/cookcli/commit/db7953425d504795fb81ff214fa175c3f75642ed))
* url separator ([88755ae](https://github.com/cooklang/cookcli/commit/88755ae3d429d8f14ba98662d8d49c64b47dc7d9))

## [0.18.1](https://github.com/cooklang/cookcli/compare/v0.18.0...v0.18.1) (2025-09-28)


### Bug Fixes

* clippy ([b860677](https://github.com/cooklang/cookcli/commit/b86067758b3e5b5d9394417a5d400fabf08aa298))
* remove discord badge ([f62b8c4](https://github.com/cooklang/cookcli/commit/f62b8c473c16c752dfedf8883596feb7340bb2f5))

## [0.18.0](https://github.com/cooklang/cookcli/compare/v0.17.2...v0.18.0) (2025-09-24)


### Features

* output latex format ([9930d99](https://github.com/cooklang/cookcli/commit/9930d99486a9535d1b63cb7991a5cc30c0098c84))
* output schema.org format ([c3beec7](https://github.com/cooklang/cookcli/commit/c3beec72f0da063b9402e11102667fbe3600f0af))

## [0.17.2](https://github.com/cooklang/cookcli/compare/v0.17.1...v0.17.2) (2025-09-19)


### Bug Fixes

* make self-update optional ([309af5e](https://github.com/cooklang/cookcli/commit/309af5e845240602aa6eef52d862fbc958070a4b))
* make self-update optional ([a0ab12b](https://github.com/cooklang/cookcli/commit/a0ab12b021bb364843cd4ceb3354d281f0a297c1))
* review ([96691aa](https://github.com/cooklang/cookcli/commit/96691aadf26df5ce99b5617fe2cb24f466e7c38d))

## [0.17.1](https://github.com/cooklang/cookcli/compare/v0.17.0...v0.17.1) (2025-09-18)


### Bug Fixes

* notarization ([aea70d2](https://github.com/cooklang/cookcli/commit/aea70d28e18a713a26f42b9a4f08de90b9a5ff7c))
* screenshots ([37914c6](https://github.com/cooklang/cookcli/commit/37914c6050a55bf252eb212f707f3d3bf679dda3))

## [0.17.0](https://github.com/cooklang/cookcli/compare/v0.16.0...v0.17.0) (2025-09-18)


### Features

* add support for keyboard ([6f75986](https://github.com/cooklang/cookcli/commit/6f7598696d7ef4f1b96bde46f1c4aed879e1eb0d))
* update command ([669899f](https://github.com/cooklang/cookcli/commit/669899f613f9fea9d6d09e5e62672b9c78954b30))


### Bug Fixes

* notarization issue ([8d9ba85](https://github.com/cooklang/cookcli/commit/8d9ba85065df3d776a2667c1d1723d53d04cb518))
* pantry integration with shop list ([85a9993](https://github.com/cooklang/cookcli/commit/85a9993c201b52f5d2101393da15a8018b657a02))

## [0.16.0](https://github.com/cooklang/cookcli/compare/v0.15.1...v0.16.0) (2025-09-16)


### Features

* add support for notes ([2afbfd9](https://github.com/cooklang/cookcli/commit/2afbfd9716211b7b21657cfced1f9c598b15456a))
* add support for sections ([de2dc6c](https://github.com/cooklang/cookcli/commit/de2dc6cf7f2f468d0d9745888e1a11342399b13f))
* dark mode ([9f6e3de](https://github.com/cooklang/cookcli/commit/9f6e3de4975ad9717cf47414e1118a69d56a6a9d))
* pantry command ([ea84f31](https://github.com/cooklang/cookcli/commit/ea84f314649c8bdf2766c85b5548b7b252193f99))
* pantry ui ([8a5867d](https://github.com/cooklang/cookcli/commit/8a5867dafdc859a5934bbb914b1afe387ad4e624))
* print recipe button ([4053236](https://github.com/cooklang/cookcli/commit/4053236525e305bf9a219f2c516dfa4021c6b46e))
* test setup ([e622860](https://github.com/cooklang/cookcli/commit/e622860ef1e373490faf9f7831470b45d46f2982))


### Bug Fixes

* ci ([4ce7af4](https://github.com/cooklang/cookcli/commit/4ce7af401a9be02d1a91e74758f13a6815ab8576))
* ci ([d3b4c77](https://github.com/cooklang/cookcli/commit/d3b4c77cc99bebbca6283c2b68f797427d9f624e))
* ci build ([0b023a3](https://github.com/cooklang/cookcli/commit/0b023a3f001f7a6656ee2a7e054892cce7d49cdd))
* pantry support now working ([62ba5e2](https://github.com/cooklang/cookcli/commit/62ba5e21ef4c28c2782f4f2900569fad50270bc3))
* shopping list goes recursive ([2e831a0](https://github.com/cooklang/cookcli/commit/2e831a05c915cc6714df361dd1210711a041ebb6))
* update actions ([26f65d6](https://github.com/cooklang/cookcli/commit/26f65d6524d45b70f0363b5d3967adb76fbce198))

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

## [0.15.0](https://github.com/cooklang/cookcli/compare/v0.14.0...v0.15.0) (2025-08-28)


### Features

* cook doctor ([d46b166](https://github.com/cooklang/cookcli/commit/d46b16607ba687501e2808703b0a9910db83a3b6))
* new UI ([e200582](https://github.com/cooklang/cookcli/commit/e2005823bd51ef9eba87859e68b4af30b8501d4a))
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
