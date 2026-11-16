# Pantry
pantry-title = 在庫一覧
pantry-intro = 家にある物を保管場所ごとに。買い物リストはすでにある物を除きます。
pantry-unreadable = パントリーファイルを読み込めませんでした
pantry-empty = 在庫はありません
pantry-filters = 表示
pantry-filter-all = すべて
pantry-filter-low = 残りわずか
pantry-filter-out = 在庫切れ
pantry-filter-expiring = 期限間近
pantry-filter-empty = この条件に合う品目はありません。
pantry-stock-ok = 在庫あり
pantry-expired = 期限切れ
pantry-expires-today = 今日まで
pantry-expires-in =
    { $count ->
       *[other] あと { $count } 日
    }
pantry-section = { $name }
pantry-no-config = 在庫の設定が見つかりません
pantry-create-config = 在庫を管理するには pantry.conf を作成してください
pantry-configure = 在庫を設定する →
pantry-manage = 在庫を管理

# Pantry Item Fields
pantry-item-name = 品名
pantry-item-quantity = 数量
pantry-item-bought = 購入:
pantry-item-bought-date = 購入日
pantry-item-expire = 期限:
pantry-item-expire-date = 賞味・消費期限
pantry-item-low = 残り少ない目安:
pantry-item-low-threshold = 残り少ないとみなす量
pantry-placeholder-quantity = 例: 500%g、2%L
pantry-placeholder-low = 例: 100%g、2

# Pantry Actions
pantry-add-item = 品目を追加
pantry-add = 追加
pantry-edit-item = 品目を編集
pantry-remove-item = 品目を削除
pantry-remove-confirm = { $section } から { $name } を削除しますか？
pantry-confirm-remove-template = %s を %s から削除しますか？
pantry-mark-low = 残り少ないにする
pantry-restock = 補充
pantry-save = 保存
pantry-cancel = キャンセル
pantry-section-label = 保管場所
pantry-section-required = 保管場所の名前を入力してください
pantry-general-quantity-only = 最初の保管場所より前の品目には数量しか設定できません。日付や下限を設定するには保管場所に移してください。
pantry-section-freezer = 冷凍庫
pantry-section-fridge = 冷蔵庫
pantry-section-pantry = 常温
pantry-section-spices = スパイス
pantry-section-general = 全般
pantry-tab-items = アイテム
pantry-tab-text = テキスト
pantry-text-intro = ファイルを直接編集します：[セクション] の行のあとに、1 行に 1 アイテムを 名前 = "数量" または 名前 = {"{"} quantity = "…", bought = "…", expire = "…", low = "…" {"}"} の形で書きます。# で始まる行はコメントです。
pantry-text-save = ファイルを保存
pantry-text-saved = 保存しました
pantry-text-conflict = 編集中にパントリーファイルが別の場所で変更されました。入力したテキストは残っています。もう一度保存すると、その変更を置き換えます。
pantry-rename-section = セクション名を変更
pantry-section-name = セクション名
pantry-failed-rename = セクション名を変更できませんでした
pantry-failed-save-file = ファイルを保存できませんでした
pantry-optional = 任意
pantry-failed-add = 品目を追加できませんでした
pantry-item-name-required = 品名を入力してください
pantry-item-exists = %s はすでに %s にあります
pantry-section-name-taken = %s という名前の保管場所または品目がすでにあります
pantry-failed-update = 品目を更新できませんでした
pantry-failed-remove = 品目を削除できませんでした
