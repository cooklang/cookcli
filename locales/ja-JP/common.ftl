# Navigation
nav-recipes = レシピ
nav-shopping-list = 買い物リスト
nav-pantry = 在庫
nav-preferences = 設定

# Search
search-placeholder = レシピを検索…
search-no-results = レシピが見つかりません

# Common Actions
action-add = 追加
action-remove = 削除
action-edit = 編集
action-save = 保存
action-cancel = キャンセル
action-back = 戻る
action-delete = 削除
action-clear = クリア
action-print = 印刷
action-preview = プレビュー
action-done = 完了

# Common Labels
label-scale = 倍率
label-servings = 人数
label-time = 時間
label-difficulty = 難易度
label-name = 名前
label-description = 説明

# Editor
editor-unsaved-changes = 未保存の変更があります
editor-saved = 保存しました
editor-saving = 保存中…
editor-save-failed = 保存できませんでした
editor-placeholder = ここにレシピを入力…

# Editor toolbar
editor-toolbar-label = Cooklang の書式
editor-toolbar-inline = 行内の要素
editor-toolbar-block = 行の要素
editor-toolbar-ingredient = 材料
editor-toolbar-ingredient-title = 材料 (@) を挿入、または選択範囲を材料にする
editor-toolbar-cookware = 調理器具
editor-toolbar-cookware-title = 調理器具 (#) を挿入、または選択範囲を調理器具にする
editor-toolbar-timer = タイマー
editor-toolbar-timer-title = 分単位のタイマー (~) を挿入、または選択範囲の名前を付ける
editor-toolbar-section = セクション
editor-toolbar-section-title = 選択範囲を名前にして新しいセクション (== セクション ==) を始める
editor-toolbar-section-default = セクション
editor-toolbar-note = メモ
editor-toolbar-note-title = 現在の行をメモ (>) にする、または手順に戻す
editor-toolbar-comment = コメント
editor-toolbar-comment-title = 現在の行 (--)、または行内の選択範囲をコメントアウトする
editor-toolbar-metadata = メタデータ
editor-toolbar-metadata-title = フロントマター (---) にメタデータの行を追加する
editor-toolbar-menu = 献立の要素
editor-toolbar-other = その他の要素
editor-toolbar-day = 日
editor-toolbar-day-title = 新しい日 (== 日 ==) を始める。日付を選ぶと日付も入る
editor-toolbar-day-default = 日
editor-toolbar-day-date = 次の日の日付
editor-toolbar-day-date-title = 次の日の日付（任意）。例: == 土曜日 (2026-03-07) ==
editor-toolbar-meal = 食事
editor-toolbar-meal-title = 最初の項目付きで食事 (朝食: \) を始める
editor-toolbar-meal-breakfast = 朝食
editor-toolbar-meal-lunch = 昼食
editor-toolbar-meal-dinner = 夕食
editor-toolbar-meal-snacks = 間食
editor-toolbar-add-recipe = レシピを追加
editor-toolbar-add-recipe-title = 現在の食事にレシピを追加する (- @./レシピ{"{}"})
editor-toolbar-recipe-reference = レシピの参照
editor-toolbar-recipe-reference-title = 別のレシピを参照する (@./レシピ{"{}"})

# Recipe picker
recipe-picker-title = レシピを選ぶ
recipe-picker-search = レシピを検索
recipe-picker-results = レシピ
recipe-picker-servings = 人数
recipe-picker-servings-hint = 空欄ならレシピに書かれた人数を使います。
recipe-picker-insert = 挿入
recipe-picker-no-results = レシピが見つかりません
recipe-picker-load-failed = レシピを読み込めませんでした

# LSP Status
lsp-connected = LSP 接続中
lsp-disconnected = 切断
lsp-error = LSP エラー

# New Recipe
new-recipe = 新しいレシピ
new-recipe-path = レシピのパス
new-recipe-filename = レシピ名
new-recipe-placeholder = 和食/煮物/肉じゃが
new-recipe-hint = 「フォルダ/レシピ名」の形で入力してください
new-recipe-create = レシピを作成

# New Menu
new-menu = 新しい献立
new-menu-path = 献立のパス
new-menu-placeholder = 献立/第12週
new-menu-hint = 「フォルダ/献立名」の形で入力してください
new-menu-create = 献立を作成

# Delete Recipe
delete-recipe = レシピを削除
delete-recipe-confirm = このレシピを削除してもよろしいですか？
delete-recipe-warning = この操作は取り消せません。

# Rename Recipe
action-rename = 名前を変更
rename-title = ファイル名を変更
rename-label = 新しい名前
rename-hint = ファイルは同じフォルダーに残ります。写真も一緒に名前が変わり、このファイルを使うレシピやメニューは新しい名前に更新されます。
rename-failed = 名前を変更できませんでした: %s
rename-skipped = 名前を変更しましたが、一部の参照は変更されていません: %s
rename-write-failed = 名前を変更しましたが、次のファイルを更新できませんでした: %s
rename-shopping-list = 買い物リストはまだ古い名前を使っています。新しいページからもう一度追加してください。

# Title Picture
picture-button = 写真
picture-title = タイトル写真
picture-none = 写真はまだありません。選ぶか、ここにドロップしてください。
picture-choose = 写真を選ぶ
picture-replace = 写真を差し替え
picture-remove = 削除
picture-remove-confirm = この写真を削除しますか？ファイルは削除され、元に戻せません。
picture-hint = JPEG・PNG・WebP に対応。JPEG で保存し、大きな写真は 2048 px に縮小します。
picture-from-metadata = このレシピの写真はメタデータの image 項目で指定されています。アップロードした写真を使うには、その行を削除してください。
picture-uploading = アップロード中…
picture-removing = 削除中…
picture-load-failed = 写真を読み込めませんでした
picture-upload-failed = アップロードできませんでした
picture-remove-failed = 写真を削除できませんでした
picture-too-large = 写真が大きすぎます。上限は 10 MB です。
picture-heif = HEIC と AVIF の写真は読み込めません。JPEG に変換してくれるスマートフォンのブラウザからアップロードするか、iPhone のカメラを「互換性優先」にしてください（設定 > カメラ > フォーマット）。
picture-for = 写真の対象
picture-step-title = 手順の写真
picture-target-step = 手順 { $step }: { $text }
picture-target-section = セクション { $section }
picture-step-note = 手順の写真は、手順の文章ではなく位置に紐づきます。このセクションで前の手順を追加・削除すると、写真は別の手順に移ります。
step-picture-add = この手順に写真を追加
step-picture-change = この手順の写真を差し替え
# Sign-in (users are managed on the server with `cook server user`)
sign-in = ログイン
sign-out = ログアウト
sign-in-intro = レシピの追加・編集・削除や、在庫・買い物リストの変更にはログインが必要です。
sign-in-username = ユーザー名
sign-in-password = パスワード
sign-in-failed = ユーザー名またはパスワードが違います。
signed-in-as = ログイン中:
role-forbidden = このアカウントではこの操作はできません。サーバーの管理者に別の権限を依頼してください。

# Errors
error-title = エラーが発生しました
error-back-home = レシピ一覧に戻る

# Icon button labels (aria-label / title)
aria-toggle-theme = テーマを切り替え
aria-keyboard-shortcuts = キーボードショートカット
aria-more-options = その他のオプション
aria-preferences = 設定
aria-dismiss = 閉じる
aria-decrease-scale = 倍率を下げる
aria-increase-scale = 倍率を上げる
aria-decrease-servings = 人数を減らす
aria-increase-servings = 人数を増やす
aria-close = 閉じる
