app-title = Gupi
menu-settings = 設定
menu-show-main = メインウィンドウ
menu-quit = Gupi を終了
startup-welcome = Gupi へようこそ
temporary-setup-required = 設定でセットアップを完了すると、一時会話を開始できます。
startup-checking = 確認中…
startup-quitting = 処理を終了しています…
settings-pi-command = Pi の実行ファイル
settings-path-help = 空欄にすると pi を自動検索します。引数なしの実行ファイルの絶対パスも指定できます。
settings-theme = 外観
settings-language = 言語
settings-save-pi = Pi のパスを保存
settings-reload = 再読み込み
settings-write-current = 適用中の設定をディスクに書き込む（編集中の内容は保持）
theme-system = システム
theme-light = ライト
theme-dark = ダーク
language-system = システム
language-english = English
language-chinese = 简体中文
recovery-config-title = 設定を確認してください
recovery-startup-title = Gupi を起動できませんでした
error-instance-startup = 設定フォルダーの権限を確認するか、ほかに起動中の Gupi を終了してから、もう一度お試しください。
recovery-pi-title = Pi を設定
recovery-confirm = 続行しますか？再読み込みに成功した場合のみ編集内容は破棄されます。リセット前に現在のファイルをバックアップします。
recovery-backup = バックアップ
error-config-read = 設定を読み込めませんでした。場所とアクセス権を確認してから再読み込みしてください。
error-config-parse = 設定の形式が正しくありません。修正したファイルを再読み込みするか、バックアップしてリセットしてください。
error-config-validation = 実行ファイル名を1つ、または引数なしの実行ファイルの絶対パスを入力してください。
error-config-write = 保存に失敗しました。編集中の内容と適用済みの設定は保持されています。設定ファイルを確認して再試行してください。
error-pi-probe = Pi を確認できませんでした。実行ファイルのパスと Node の環境を確認して、もう一度お試しください。
action-check-pi = Pi を再確認
action-reset = バックアップしてリセット
action-confirm = 確認
action-cancel = キャンセル
action-save = 保存
home-ready = Pi を利用できます
home-description = このバージョンでは環境設定とデスクトップ設定を利用できます。会話機能は次の段階で提供予定です。
action-locate = 設定フォルダーを表示
error-pi-timeout = Pi のバージョン確認が15秒以内に完了しませんでした。
error-pi-output = Pi の出力がバージョン確認の上限を超えました。
error-pi-version = Pi が正常に終了しなかったか、有効なバージョンを返しませんでした。
error-log = ファイルへのログ記録を利用できません。ログの保存先とアクセス権を確認してください。

setup-tagline = Pi のためのネイティブデスクトップ
setup-intro = 簡単な設定ですぐに始められます。
setup-start = 始める
setup-language-title = 使いやすい言語を選択
setup-search-language = 言語を検索…
setup-system-language = システム言語：{ $language }
setup-appearance-title = 外観をカスタマイズ
setup-color-mode = カラーモード
setup-system-accent = システムのアクセントカラー
setup-pi-title = Pi に接続
setup-pi-step = Pi のセットアップ
setup-pi-help = ローカルの Pi を確認するか、このページをスキップして後から設定できます。
setup-check-pi = Pi を確認
setup-back = 戻る
setup-next = 続行
setup-finish = セットアップを完了
setup-saving = 保存中…
light-themes = ライトテーマ
dark-themes = ダークテーマ

# Conversation workspace
conversation-new = 新しい会話
conversation-sidebar = 会話サイドバー
conversation-history = 会話履歴
conversation-restoring = 下書きを復元中…
conversation-discovering = 会話を検索中… ファイル数 { $count }
conversation-read-progress = ファイルを読み込み中 { $completed} / { $total}
conversation-refresh-progress = 更新中 { $completed} / { $total}
conversation-project = 作業フォルダーを選択
conversation-project-search-placeholder = プロジェクトを検索
conversation-project-search-empty = 一致するプロジェクトはありません
conversation-open-folder = フォルダーを開く…
conversation-untitled = 無題の会話
conversation-idle = 待機中
conversation-loading = セッションを開いています
conversation-running = 実行中
conversation-failed = エラーが発生しました
conversation-waiting = 入力待ち
conversation-search = 会話を検索
conversation-refresh = セッションを更新
conversation-refresh-current = 現在のセッションを更新
conversation-actions = セッションの操作
conversation-show-sidebar = セッションサイドバーを表示
conversation-hide-sidebar = セッションサイドバーを非表示
conversation-scan-failed = 会話カタログを読み込めませんでした
conversation-search-placeholder = 名前、メッセージ、プロジェクトのパスを検索
conversation-search-empty = 一致する会話はありません
conversation-rename = 名前を変更…
conversation-delete = ゴミ箱に移動
conversation-delete-failed = 会話を削除できませんでした。詳細を確認してから再試行してください。
conversation-copy-path = パスをコピー
conversation-clone = 会話を複製
conversation-export = 会話を HTML として書き出す…
conversation-exported = 会話を { $path} に書き出しました
conversation-stop = 生成を停止
conversation-close-run = ランタイムを閉じる
conversation-show-less = 折りたたむ
conversation-show-more = さらに表示…
conversation-continue = ここから続ける
conversation-fork = ここから会話を分岐
conversation-preview = 別のブランチをプレビュー中
conversation-return-current = 現在のブランチに戻る
conversation-welcome = 会話を始めましょう
conversation-welcome-project = { $project} で何をしますか？
conversation-empty-history = この会話にはまだメッセージがありません。
conversation-bottom = 一番下へ移動
conversation-copy = コピー
conversation-compaction = コンテキストの圧縮概要
conversation-branch-summary = ブランチの概要
conversation-working = 処理中…
conversation-process = プロセスを表示
conversation-details = 詳細を切り替え
tool-detail-offset = 開始行：
tool-detail-line-limit = 要求された行数：
tool-detail-timeout = タイムアウト（秒）：
tool-detail-output = 出力
tool-detail-input = 入力
tool-detail-additional = 追加情報
tool-detail-error = エラー
tool-detail-glob = ファイルパターン：
tool-detail-ignore-case = 大文字と小文字を区別しない：
tool-detail-literal = リテラル検索：
tool-detail-context = 前後に表示する行数：
tool-detail-result-limit = 要求された結果数：
tool-detail-old-text = 置換前のテキスト
tool-detail-new-text = 置換後のテキスト
tool-detail-truncated = Pi の出力は一部省略されています。表示内容は全体ではありません。
tool-detail-lines-truncated = 一部の結果行が短縮されました。
tool-detail-match-limit = 一致数の上限に達しました：
tool-detail-results-limited = 結果数の上限に達しました：
tool-detail-entries-limited = ディレクトリエントリ数の上限に達しました：
tool-detail-full-output = 出力ファイル全体：
tool-detail-image-unavailable = この画像を表示できません。
conversation-role-user = ユーザーメッセージ
conversation-role-assistant = アシスタントのメッセージ
conversation-role-tool = ツールの結果
conversation-event = セッションイベント
conversation-close-history = 会話履歴を閉じる
conversation-source = 元のセッションファイルを表示
conversation-reconnect = 再接続
conversation-interrupted = 実行を停止しました。既存の内容は保持されています。
conversation-no = いいえ
conversation-submit = 送信
conversation-input = メッセージを入力
conversation-model = モデル
conversation-load-options = モデルを読み込むには Pi に接続してください
conversation-thinking = 思考レベル
conversation-unknown = 利用できません
conversation-context = 現在の実行ブランチのコンテキスト
conversation-auto-compaction = 自動圧縮
conversation-on = オン
conversation-off = オフ
conversation-tokens = 入力 / 出力の合計
conversation-cache = キャッシュ読み込み / 書き込みの合計
conversation-cache-hit = 最新のキャッシュヒット率
conversation-cost = Pi が報告したコスト
conversation-statistics = 使用状況
conversation-send = 送信（Enter）；Alt+Enter で追加の依頼をキューに追加
conversation-sending = 送信中…
conversation-graph-current = 現在
conversation-graph-preview = プレビュー
conversation-graph-left = 左側のレーンを表示（または横にスクロール）
conversation-graph-right = 右側のレーンを表示（または横にスクロール）
conversation-graph-reveal = 選択項目を表示
conversation-graph-column = グラフ
conversation-graph-message = メッセージ
conversation-catalog-empty = 会話はまだありません

conversation-thinking-content = 思考
conversation-thinking-running = 思考中…
conversation-tool-group = ツール呼び出し（{ $count }）
conversation-tool-group-read = ファイルを読み込み（{ $count }）
conversation-tool-group-bash = コマンド（{ $count }）
conversation-tool-group-search = 検索（{ $count }）
conversation-tool-group-edit = ファイルの変更（{ $count }）
conversation-tool-line = { $action } { $summary }
conversation-tool-action-read = { $state ->
    [running] 読み込み中
    [complete] 読み込み済み
    [failed] 読み込みに失敗
   *[unfinished] 読み込み未完了
}
conversation-tool-action-write = { $state ->
    [running] 書き込み中
    [complete] 書き込み済み
    [failed] 書き込みに失敗
   *[unfinished] 書き込み未完了
}
conversation-tool-action-edit = { $state ->
    [running] 編集中
    [complete] 編集済み
    [failed] 編集に失敗
   *[unfinished] 編集未完了
}
conversation-tool-action-bash = { $state ->
    [running] 実行中
    [complete] 実行済み
    [failed] コマンドに失敗
   *[unfinished] コマンド実行未完了
}
conversation-tool-action-search = { $state ->
    [running] 検索中
    [complete] 検索済み
    [failed] 検索に失敗
   *[unfinished] 検索未完了
}
conversation-tool-action-other = { $state ->
    [running] { $name} を呼び出し中
    [complete] { $name} を呼び出し済み
    [failed] { $name} の呼び出しに失敗
   *[unfinished] { $name} の呼び出し未完了
}

conversation-processed = { $duration} 実行しました
conversation-processed-failed = { $duration} 後に失敗
conversation-processed-stopped = { $duration} 後に停止
conversation-copied = コピーしました
conversation-copy-failed = コピーできませんでした。もう一度お試しください。
conversation-usage-title = リクエストの使用量
conversation-usage-model = モデル
conversation-usage-provider = プロバイダー
conversation-usage-input = 入力トークン
conversation-usage-output = 出力トークン
conversation-usage-cache-read = キャッシュ読み込みトークン
conversation-usage-cache-write = キャッシュ書き込みトークン
conversation-usage-total = 合計トークン
conversation-usage-cost = コスト

composer-context-used = 使用済みトークン
composer-context-limit = コンテキスト容量
composer-context-percent = コンテキストの使用量
composer-token-input = 入力トークンの合計
composer-token-output = 出力トークンの合計
composer-token-cache-read = キャッシュ読み込みトークンの合計
composer-token-cache-write = キャッシュ書き込みトークンの合計
composer-token-usage = セッションのトークン使用量

conversation-model-search = モデルを検索…
conversation-model-empty = 一致するモデルはありません
conversation-thinking-off = オフ
conversation-thinking-minimal = 最小
conversation-thinking-low = 低
conversation-thinking-medium = 中
conversation-thinking-high = 高
conversation-thinking-xhigh = 非常に高い
conversation-thinking-max = 最大

conversation-model-reasoning = 推論
conversation-model-vision = 画像認識

conversation-history-reply = アシスタントの返信
conversation-history-brief = 簡易：ユーザーメッセージと最終回答
conversation-history-detailed = 詳細：メッセージ、ツール、概要
conversation-history-progress = アシスタントの進捗
conversation-history-thinking = 思考
conversation-history-calls = ツール呼び出し
conversation-history-failed = 実行に失敗
conversation-history-stopped = 中断
conversation-history-empty = テキストのない返信
composer-model-thinking = モデルと思考レベル
composer-model-refresh = モデルと思考レベルを再読み込み
composer-thinking-unavailable = 非対応
composer-model-loading = モデルを読み込み中…
composer-thinking-loading = 思考レベルを読み込み中…
composer-model-confirming = モデル設定を確認中…
composer-stats-loading = 使用状況を読み込み中…
composer-stats-retry = 使用状況の読み込みを再試行
conversation-checking-file = 会話ファイルを確認中…
conversation-connecting = Pi に接続中…
conversation-history-loading = 会話履歴を読み込み中…
conversation-history-refreshing = 会話履歴を更新中…
conversation-fork-options-loading = 分岐のオプションを読み込み中…
conversation-fork-options-retry = 分岐オプションの読み込みを再試行
history-canvas-zoom-in = 拡大
history-canvas-zoom-out = 縮小
history-canvas-fit = ツリー全体に合わせる（0）
history-canvas-current = 実行ノードに移動
history-canvas-expand = ノードを { $count } 件展開
history-canvas-collapse = 移動位置を残して { $count } 個のプロセスノードを折りたたむ
history-canvas-empty = 表示する履歴ノードがありません
history-canvas-help = 会話ツリー：スクロールまたはドラッグで移動、ピンチまたは +/- キーで拡大縮小、矢印キーで選択、Enter でプレビュー、E で次の区間を展開

history-view-tree = ツリー
history-view-list = リスト
history-level-brief = 簡易
history-level-detailed = 詳細
history-level-all = すべて
history-level-all-description = すべて：設定、ラベル、カスタム記録を含む
history-scope-all = すべてのブランチ
history-scope-branch = このブランチのみ
history-model-change = モデルの変更
history-thinking-change = 思考レベルの変更
history-session-info = セッション情報の変更
history-label-change = ラベルの変更
history-custom-record = カスタム記録


history-content = 内容
history-range = 範囲

command-palette = コマンドパレット
command-focus-input = 会話入力欄にフォーカス
command-model = モデルと思考レベルを選択…
command-copy-last-answer = 直前の回答をコピー
command-compact = コンテキストを圧縮
conversation-compacting = コンテキストを圧縮中…
command-history-description = 履歴を表示するか、分岐元のユーザーメッセージを選択
command-show-history = 履歴を表示
command-hide-history = 履歴を非表示
command-current-session = 現在のセッション：
command-unavailable = 現在の状態では利用できません
command-target-changed = この会話は利用できなくなりました。別の会話を選択してください。
command-scanning = 会話を読み込み中…
command-loading = Pi のコマンドを読み込み中…
command-no-matches = 一致する Pi コマンドはありません
conversation-reconnect-unconfirmed = Pi プロセスの終了を確認できませんでした。再接続する前に Gupi を再起動してください。
action-retry = 再試行

command-search-placeholder = アクションを検索するか、コマンドを入力
command-empty = このセッションでは Pi コマンドを利用できません

command-group-app = アプリケーション
command-group-extensions = 拡張機能のコマンド
command-group-skills = スキル
command-group-prompts = プロンプトテンプレート
command-scope-user = 個人
command-scope-project = プロジェクト
command-scope-temporary = 一時
command-connection-unavailable = Pi はまだ準備できていません
command-dismiss = 閉じる
command-complete = 完了
command-send-text = 送信
command-execute = 実行

command-scope-current = 現在のセッション

conversation-working-duration = 作業中 { $duration }
conversation-tool-group-skill = 読み込んだスキル（{ $count }）
conversation-shell-line = { $shell} · { $action } { $summary}
conversation-tool-action-skill = { $state ->
    [running] スキルを読み込み中
    [complete] スキルを読み込み済み
    [failed] スキルの読み込みに失敗
   *[unfinished] スキルの読み込み未完了
}

# Unified settings
settings-page-general = 一般
settings-page-pi = Pi
settings-page-keys = キーボードショートカット
settings-page-skills = スキル
settings-page-prompts = プロンプト
settings-page-about = このアプリについて
settings-key-reset = 既定に戻す
settings-key-reset-all = すべての既定値に戻す
settings-key-reset-all-confirm = すべてのショートカットを既定の割り当てに戻しますか？カスタムの割り当てはすべて削除されます。
settings-key-clear = ショートカットを解除
settings-key-cancel = 変更を取り消す
settings-key-unbound = ショートカットが割り当てられていません
settings-key-recording = ショートカットを押してください（Esc でキャンセル）
settings-key-conflict = ショートカットが競合しているか、無効です
settings-key-invalid = 有効なショートカットを入力するか、空欄にして割り当てを解除してください
settings-resource-refresh = 再読み込み
settings-resource-reload-help = 個人用リソースのみです。変更を適用するには既存のセッションを手動で再読み込みしてください。
settings-resource-working = 処理中…
settings-resource-pi-required = まず Pi の設定で、Pi の実行ファイルを保存して動作確認してください。
settings-resource-invalid-name = パス区切り文字を含まない有効なファイル名を入力してください。
settings-resource-register = ローカルに追加
settings-resource-create = 作成
settings-resource-empty = 一致するリソースはありません。
settings-resource-delete = ゴミ箱に移動
settings-resource-save = テキストを保存
shortcut-save-task = 保存
settings-resource-loading = テキストを読み込み中…
settings-resource-confirm-remove = 選択したファイルだけをゴミ箱に移動し、ディレクトリは残します。
settings-package-install = パッケージをインストール
settings-package-update = 更新
settings-package-remove = 削除
settings-package-source-help = パッケージの取得元：npm:name、Git URL、またはローカルの絶対パス。
settings-package-extensions = 拡張機能 { $count}
settings-package-skills = スキル { $count}
settings-package-prompts = テンプレート { $count}
settings-package-themes = テーマ { $count}
settings-skill-search = 名前、説明、パス、取得元でスキルを検索
settings-skill-collapse = 折りたたむ
settings-resource-edit = 編集…
settings-about-gupi = バージョン
settings-about-pi = バージョン
settings-about-path = 解決済みのパス
settings-about-status = 確認状況
settings-about-unavailable = まだ利用できません
settings-editor-discard = 保存されていないテキストの変更を破棄しますか？
settings-package-confirm-remove = 選択した Pi スコープから削除します。ローカルのソースフォルダーは残ります。
settings-package-heading = インストール済みパッケージ
settings-extension-heading = 単独の拡張機能
settings-template-heading = コマンドテンプレート
settings-system-heading = システムプロンプト
settings-config-heading = 設定ファイル
settings-config-open = 設定ファイルを開く
settings-mode-help = システムに合わせて、ライトテーマとダークテーマを自動的に切り替えます。
settings-light-help = ライトモードで使用します。
settings-dark-help = ダークモードで使用します。
settings-pi-unsaved = パスは未保存です。確認すると現在の入力を検証しますが、保存はされません。
settings-pi-saved = 現在のパスは保存されています。Pi が利用できることをもう一度確認してください。
settings-resource-location = 場所を表示
settings-package-source = パッケージの取得元
settings-package-empty = 個人用の Pi パッケージはインストールされていません。
settings-package-resources-empty = このパッケージで宣言されたリソースは見つかりません。
settings-resource-name = 名前
settings-resource-none = この種類の個人用リソースはまだありません。
settings-resource-view = 表示
settings-resource-readonly-badge = 読み取り専用
settings-source-personal = 個人
settings-source-external = 外部
settings-system-replace = 既定のシステムプロンプトを置き換える
settings-system-replace-help = Pi の既定のシステムプロンプトを完全に置き換えます。
settings-system-append = 個人用の指示を追加
settings-system-append-help = 既定のシステムプロンプトを維持し、その後に指示を追加します。
settings-resource-success = 「{ $target}」に対する操作が完了しました。既存のセッションを手動で再読み込みしてください。
settings-resource-failed = 「{ $target}」に対する操作に失敗しました。
settings-about-help = 保存済みパスの最新の確認結果を表示します。もう一度確認するには Pi の設定を開いてください。
settings-key-reveal-session = セッションファイルを表示

settings-key-group-app = アプリケーション
settings-key-group-conversation = 会話
settings-key-group-files = セッションファイル

settings-package-kind-extensions = 拡張機能
settings-package-kind-skills = スキル
settings-package-kind-themes = テーマ
settings-package-kind-prompts = テンプレート
settings-package-expand = パッケージの内容を展開
settings-package-collapse = パッケージの内容を折りたたむ

settings-template-search = テンプレート名、説明、取得元を検索
settings-template-collapse = プレビューを折りたたむ

temporary-title = 一時会話
attachment-add = ファイルを添付
attachment-remove = 添付を削除
attachment-file = ファイル
attachment-clipboard = クリップボードの画像
image-preview-close = プレビューを閉じる
image-preview-open = 画像のプレビューを開く
image-preview-zoom-in = 拡大
image-preview-zoom-out = 縮小
shortcut-global = グローバルショートカット
shortcut-launcher = 一時ウィンドウ
shortcut-add = テンプレートタスクを追加…
shortcut-edit = テンプレートタスクを編集…
shortcut-delete = テンプレートタスクを削除
shortcut-name = 名前
shortcut-template = コマンドテンプレート
shortcut-source = 入力元
shortcut-selection = 選択したテキスト
shortcut-clipboard = クリップボード
shortcut-fallback = 選択したテキスト。ない場合はクリップボード
shortcut-model = モデル
shortcut-thinking = 思考レベル
shortcut-default-model = Pi の既定モデル
shortcut-default-thinking = Pi の既定の思考レベル
shortcut-personal = 個人用
shortcut-template-required = コマンドテンプレートを選択してください。
shortcut-reload-options = オプションを再読み込み
temporary-prewarm = 一時会話を事前準備
temporary-prewarm-help = 新しい一時会話を素早く開始できるよう、Pi プロセスをバックグラウンドで1つ待機させます。実測メモリ使用量は拡張機能により約60 MBで、拡張機能の初期化、ログイン状態の更新、ネットワークリクエストを事前に実行します。
temporary-clean-released = 解放済みのワークスペース
temporary-clean = クリーンアップ
temporary-clean-help = 解放済みの一時ワークスペースをゴミ箱に移動します。実行中の会話は保持されます。
temporary-cleaned = ワークスペースをゴミ箱に移動しました
shortcut-tasks = テンプレートタスク

shortcut-select-template = プロンプトテンプレートを選択

temporary-search-placeholder = 一時会話を検索
temporary-search-empty = 一致する一時会話はありません
temporary-toggle-input = 入力欄を切り替え
temporary-hide = 非表示
session-search-open = 会話を開く

temporary-actions = 操作
temporary-paste-answer = 直前の回答を貼り付け
temporary-reveal-workspace = 作業ディレクトリを表示
temporary-search-actions = 操作を検索…
temporary-switch-session = 一時会話を切り替え
temporary-paste-failed = 回答をコピーしました。自動貼り付けに失敗しました。アクセシビリティ権限と貼り付け先を確認するか、手動で貼り付けてください。
temporary-session-1 = 一時会話 1 に切り替え
temporary-session-2 = 一時会話 2 に切り替え
temporary-session-3 = 一時会話 3 に切り替え
temporary-session-4 = 一時会話 4 に切り替え
temporary-session-5 = 一時会話 5 に切り替え
temporary-session-6 = 一時会話 6 に切り替え
temporary-session-7 = 一時会話 7 に切り替え
temporary-session-8 = 一時会話 8 に切り替え
temporary-session-9 = 最後の一時会話に切り替え
temporary-send = 送信
temporary-trash = 一時会話をゴミ箱に移動
settings-key-stop-or-hide = 生成を停止 / 一時ウィンドウを非表示

conversation-queue-title = キューに追加済み（{ $count}）
conversation-queue-steer = 現在のターンに指示
conversation-queue-follow-up = 追加タスク
conversation-queue-restore = キュー内のテキストをすべて下書きに戻す
conversation-queue-clear = キュー内のメッセージをすべて消去
conversation-queue-no-text = テキストのないメッセージ
conversation-queue-unavailable = キューの内容はまだ利用できません
conversation-queue-text-only = キューから戻すとテキストのみ復元され、画像は復元されません。

conversation-retry-countdown = 約 { $seconds} 秒後に再試行（{ $attempt}/{ $total}）
conversation-retry-waiting = 再試行（{ $attempt}/{ $total}）：Pi を待っています
conversation-summary-retry-countdown = 概要を約 { $seconds} 秒後に再試行（{ $attempt}/{ $total}）
conversation-summary-retry-waiting = 概要を再試行（{ $attempt}/{ $total}）：Pi を待っています

message-details-open = 詳細を表示…
message-details-copy-all = すべてコピー
tool-detail-command = コマンド
tool-detail-content = 内容
tool-detail-changes = 変更

settings-notifications = 通知
settings-notification-help = システム通知には一般的な説明が表示されます。通知を開くと会話に戻ります。会話が前面に表示されると未読数が消去されます。
settings-notification-waiting = バックグラウンドで入力が必要なときに通知
settings-notification-failures = バックグラウンドでタスクが失敗したときに通知
settings-notification-plugins = プラグインのリマインダーをシステム通知で送信
settings-notification-attention = バックグラウンドで入力が必要なときに注意を促す
settings-notification-completion = 回答完了の通知
notification-off = オフ
notification-background = バックグラウンドのみ
notification-always = 常に表示
notification-waiting = 入力を待っています
notification-completed = 回答の準備ができました
notification-failed = タスクが失敗し、確認が必要です
notification-plugin = プラグインのリマインダー
notification-unread = 未読の会話
notification-running = 実行中
notification-clear = リマインダーを消去
notification-session-notices = 会話のリマインダー

settings-icon-theme = アプリアイコン
settings-icon-theme-help = アプリ内ロゴと、実行中アプリの macOS Dock アイコンを変更します。Finder と Windows タスクバーでは同梱アイコンが使われます。
icon-theme-classic = クラシック
icon-theme-classic-gradient = クラシック（グラデーション）
icon-theme-color = Pi カラー
icon-theme-color-gradient = Pi グラデーション
icon-theme-pride = レインボー
icon-theme-ukraine = ウクライナ
icon-theme-ukraine-gradient = ウクライナ（グラデーション）

language-traditional-chinese = 繁體中文
language-japanese = 日本語
language-korean = 한국어
language-german = Deutsch
language-french = Français
language-spanish = Español
language-portuguese-brazil = Português (Brasil)
menu-about = Gupi について
menu-conversation = 会話
menu-edit = 編集
menu-view = 表示
menu-window = ウィンドウ
menu-help = ヘルプ
menu-services = サービス
menu-hide = Gupi を隠す
menu-hide-others = ほかを隠す
menu-show-all = すべて表示
menu-undo = 取り消す
menu-redo = やり直す
menu-cut = 切り取り
menu-copy = コピー
menu-paste = 貼り付け
menu-select-all = すべて選択
menu-minimize = 最小化
menu-zoom = 拡大／縮小
menu-fullscreen = フルスクリーンを切り替え
menu-docs = Gupi ユーザーガイド
menu-pi-docs = Pi ドキュメント
menu-report-issue = 問題を報告
menu-logs = ログを表示
menu-copy-diagnostics = 診断情報をコピー
settings-permissions = システム権限
settings-accessibility-help = 選択したテキストの読み取りや別のアプリへの貼り付けには、アクセシビリティの許可が必要な場合があります。システム設定で Gupi を許可してから、もう一度操作してください。貼り付けに失敗しても回答はコピーできます。
settings-accessibility-open = アクセシビリティ設定
settings-notification-permission-help = 通知や Dock バッジが表示されない場合は、システム設定で Gupi の通知とバッジを許可してください。この機能を初めて使うときに許可を求めます。
settings-notification-permission-open = 通知設定
settings-native-language-help = アプリの言語はすぐに切り替わります。macOS のシステムダイアログに適用するには Gupi を再起動してください。Windows のシステムダイアログは Windows の表示言語に従います。
settings-diagnostics-help = アプリのバージョン、プラットフォーム、Pi コマンド、設定とログの保存場所が含まれます。会話、認証情報、環境変数は含まれません。

setup-preferences-title = 言語と外観
setup-preferences-step = 言語と外観
setup-desktop-title = ショートカットと通知
setup-desktop-step = ショートカットと通知
setup-desktop-help = これらの設定は後から設定で変更できます。
setup-page-skip = このページをスキップ
setup-skip-all = 既定値を使用
setup-pi-later = Pi を後から設定
action-open = 開く

conversation-plugin-message = プラグインメッセージ
conversation-session-info-command = 会話情報…
conversation-session-info-title = 会話情報
conversation-session-info-identity = 識別情報と場所
conversation-session-info-name = 会話
conversation-session-info-id = Pi 会話 ID
conversation-session-info-directory = 作業ディレクトリ
conversation-session-info-file = 会話ファイル
conversation-session-info-not-saved = 未保存
conversation-session-info-runtime = 実行設定
conversation-session-info-model = モデル
conversation-session-info-thinking = 思考レベル
conversation-session-info-usage = 使用量
conversation-session-info-input = 入力トークン
conversation-session-info-output = 出力トークン
conversation-session-info-cache-read = キャッシュ読み込み
conversation-session-info-cache-write = キャッシュ書き込み
conversation-session-info-cost = Pi の報告費用
conversation-session-info-context = 現在のコンテキスト（トークン / 上限 / 使用率）
conversation-session-info-context-value = 使用 { $tokens } / 上限 { $window } トークン（{ $percent }）
conversation-session-info-counts = 会話の統計
conversation-session-info-user-messages = ユーザーメッセージ
conversation-session-info-assistant-messages = アシスタントメッセージ
conversation-session-info-tool-calls = ツール呼び出し
conversation-session-info-tool-results = ツール結果
conversation-session-info-total-messages = メッセージ合計

setup-appearance-options = テーマとアイコンを選択
setup-pi-install-guide = Pi をインストール
setup-pi-model-guide = モデルに接続

conversation-find = 会話内を検索
conversation-find-placeholder = 会話内を検索
conversation-find-count = { $current } / { $total }
conversation-find-previous = 前の一致（Shift+Enter）
conversation-find-next = 次の一致（Enter）
conversation-find-close = 検索を閉じる（Esc）

# Composer resources
command-select-resource = メッセージに挿入
command-group-files = ファイルとフォルダー
resource-files-loading = プロジェクトのファイルとフォルダーを検索中…
resource-open-file = ファイルを開く
resource-skill-content = Skill の指示
resource-template-unreadable = プロンプトテンプレートを読み込めません。リソースを再読み込みして、もう一度お試しください。
resource-template-unused-files = このテンプレートでは、参照ファイルの引数がすべて使われていません。不足しているパラメーター（例：$ARGUMENTS）を追加するか、参照を削除してから送信してください。
resource-search-files = プロジェクトのファイルとフォルダーを検索…
resource-choose-files = ファイルまたはフォルダーを選択…

resource-edit-in-composer = ここでリソースを選び、引数はメッセージ入力欄で編集します。
resource-template-unclosed-quote = ファイル参照を送信する前に、テンプレート引数の引用符を閉じてください。

resource-answer-required = 回答を選択してから送信してください。

menu-check-updates = アップデートを確認…
updates-title = アップデート
updates-check = アップデートを確認
updates-automatic = アップデートを自動で確認
updates-automatic-help = 毎時間 GitHub で新しいバージョンを確認します。ダウンロードは手動で開始します。
updates-idle = Gupi の新しいバージョンがあるか確認します。
updates-checking = アップデートを確認中
updates-current = 最新バージョンです
updates-unpublished = 安定版はまだ公開されていません。
updates-available = Gupi { $version } が利用可能です
updates-release = リリースノートとダウンロード
updates-install-help = Gupi のインストールに使用したインストーラーまたはパッケージマネージャーで更新してください。
updates-error-network = アップデートを確認できませんでした。接続を確認して再試行してください。
updates-error-rate-limit = GitHub のリクエスト上限に達しました。しばらくしてから再試行してください。
updates-error-response = リリース情報を読み取れませんでした。しばらくしてから再試行してください。

updates-install = ダウンロードしてインストール…
updates-native-progress = 更新ウィンドウで更新を完了してください。
updates-install-failed = 更新を完了できませんでした。再試行するか、Gupi を開き直してログを確認してください。
updates-restart-help = インストール時に下書きを保存し、Pi 接続を閉じて Gupi を再起動します。
updates-show-progress = 更新の進行状況を表示…

shortcut-screenshot = スクリーンショット
screenshot-failed = 画面をキャプチャできませんでした。もう一度お試しください。
screenshot-permission-required = スクリーンショットには画面収録の許可が必要です。「システム設定 → プライバシーとセキュリティ → 画面収録」で Gupi を許可してから、もう一度お試しください。macOS から再起動を求められた場合は、先に Gupi を再起動してください。

# Project files and source preview
files-title = ファイル
files-panel = ワークスペース
files-close = パネルを閉じる
files-refresh = 再読み込み
files-preview = プレビュー
files-readonly = 読み取り専用
files-wrap = 折り返し
files-reload = 再読み込み
files-loading = 読み込み中…
files-expanded = 展開済み
files-collapsed = 折りたたみ
files-incomplete = 一部の項目を読み込めません。再試行
files-read-failed = ファイルまたはフォルダーを読み込めません。再試行
files-unsupported = このファイルはプレビューできません
files-too-large = プレビュー上限（1 MiB または20,000行）を超えています
files-long-line = 長い行があるため構文強調を無効にしました
files-no-session = 会話を選択してプロジェクトのファイルを表示

files-actions = ファイル操作
files-copy-path = パスをコピー
files-reveal = ファイル一覧で表示
files-close-source = ファイルを閉じる
files-focus-source = ソースにフォーカス
files-conversation = 会話
files-conversation-busy = 会話、応答中
files-view = 表示
files-source = ソース

files-link-label = { $name } — シンボリックリンク
files-link-directory-label = { $name } — フォルダーリンク、{ $state }
files-link-unavailable-label = { $name } — リンク先を利用できません
files-link-unavailable = リンク先を利用できません

# Settings and shortcuts redesign
settings-page-packages = パッケージ
settings-app-language = 表示言語
settings-config-location = 場所
settings-temporary-storage = 一時会話の保存
settings-accessibility = アクセシビリティ
settings-open-system-settings = システム設定を開く
settings-color-mode-label = モード
settings-light-help-active = ライトモードで使用します。現在使用中です。
settings-dark-help-active = ダークモードで使用します。現在使用中です。
settings-icon-theme-help-other = アプリ内のロゴを変更します。タスクバーとファイルマネージャーはパッケージのアイコンのままです。
settings-notification-background-heading = バックグラウンドの動作
settings-notification-completion-heading = 回答の完了
settings-notification-permission-help-windows = 通知が届かない場合は、Windows の設定で Gupi の通知を許可してください。権限は機能を初めて使うときに求められます。
settings-pi-status-ready = 利用可能 · { $version } · { $path }
settings-pi-draft = 未保存
settings-pi-saved-tag = 保存済み
settings-diagnostics = 診断
settings-open-pi = Pi の設定を開く
settings-resource-actions = その他の操作
settings-resource-delete-ellipsis = ゴミ箱に移動…
settings-resource-from-package = { $package } から
settings-resource-open-package = パッケージを表示
settings-key-customized = カスタム
settings-key-customized-only = カスタムのみ
settings-key-group-system = システム全体
settings-key-system-help = Gupi の実行中は、他のアプリを使っているときもこれらのショートカットが使えます。
settings-key-group-workspace = ワークスペースとファイル
settings-key-reset-all-ellipsis = すべての既定値に戻す…
settings-key-confirm = 確定
settings-key-unsaved = 未保存
settings-key-scope-any-window = すべての Gupi ウインドウ
settings-key-scope-palette = メイン、一時、起動の各ウインドウ
settings-key-scope-conversation = メインと一時ウインドウの会話
settings-key-scope-messages = メッセージ一覧にフォーカスがあるとき
settings-key-scope-stop = メイン：生成を停止 · 一時：隠す
settings-key-scope-temporary = 一時ウインドウ
settings-key-scope-temporary-paste = 一時ウインドウ。Enter のみの場合は入力欄では動作しません
settings-key-search = ショートカットを検索
settings-key-search-keystroke = キーで検索
settings-key-search-clear = キー検索を解除
settings-key-sessions = 一時会話 1–9 に切り替え
settings-key-sessions-expand = すべて表示
settings-key-sessions-collapse = 折りたたむ
settings-key-show-conflict = 表示
settings-key-used-by = 「{ $command }」（{ $scope }）で使用中です
settings-key-task-actions = タスクの操作
settings-key-task-description = /{ $template } · { $source }
settings-key-task-enabled = タスクを有効にする
shortcut-delete-ellipsis = テンプレートタスクを削除…

# Package catalog
packages-installed-tab = インストール済み
packages-browse-tab = 探す
packages-search = パッケージを検索
packages-source-npm = 結果は npm で「pi-package」キーワードが付いたパッケージです。
packages-directory = Pi パッケージディレクトリ
packages-loading = 読み込み中…
packages-retry = 再試行
packages-installed = インストール済み
packages-previous = 前へ
packages-next = 次へ
packages-back = 戻る
packages-latest = 最新の公開バージョン { $version }
packages-publisher = 公開者 { $publisher }
packages-repository = リポジトリ
packages-npm = npm ページ
packages-install-source = インストール元
packages-declared = 宣言された内容
packages-undeclared = パッケージは内容を宣言していません。インストール後に一覧表示されます。
packages-installed-version = { $version } をインストール済み
packages-files-missing = ファイルがありません
packages-trust = パッケージはあなたの権限でコードを実行します。
packages-done = 完了しました。既存のセッションを再読み込みするか、新しいセッションで反映されます。
packages-failed = Pi がエラーを報告しました：
packages-install = インストール
packages-copy-source = インストール元をコピー
packages-open-repository = リポジトリを開く
packages-open-npm = npm ページを開く
packages-problem-network = npm レジストリに接続できません。接続を確認して再試行してください。
packages-problem-rate-limited = npm レジストリがリクエストを制限しています。しばらくしてから再試行してください。
packages-problem-not-found = パッケージが見つかりません。
packages-problem-invalid = npm レジストリから予期しない応答が返されました。
settings-package-install-source = ソースからインストール…
settings-package-remove-ellipsis = 削除…

# Package browser layout
packages-source-short = npm · pi-package
packages-page = { $page } ページ
packages-page-empty = このページに条件に一致するパッケージはありません。
packages-close = 詳細を閉じる
packages-local-unavailable = ローカルパッケージ情報を取得できません。操作するにはリソースを再読み込みしてください。
packages-refresh-details = 再読み込み

## Pi settings
settings-domain-label = 設定の領域
settings-domain-gupi = Gupi
settings-domain-pi = Pi
settings-page-connection = 接続
settings-page-pi-conversation = 会話
settings-page-pi-network = ネットワーク
pi-group-model = デフォルトモデル
pi-group-context = コンテキスト
pi-group-queue = メッセージキュー
pi-group-images = 画像
pi-group-proxy = プロキシ
pi-group-retry = 再試行
pi-field-model = デフォルトモデル
pi-field-model-help = 新しい会話の既定モデル。モデル範囲によって別の初期モデルが選ばれる場合があります。
pi-field-model-auto = 自動
pi-field-model-unavailable = このモデルは現在利用できません。
pi-field-thinking = デフォルトの思考レベル
pi-thinking-unsupported = { $level }（このモデルは非対応）
pi-field-compaction = 自動圧縮
pi-field-reserve = 応答用に確保するトークン
pi-field-reserve-help = 圧縮時にモデルの応答用として空けておくトークン数です。
pi-field-keep = 保持する最近のトークン
pi-field-keep-help = 圧縮で要約されない最近のコンテキストです。
pi-field-steering = 割り込みメッセージ
pi-field-follow-up = フォローアップメッセージ
pi-delivery-one = 1 件ずつ
pi-delivery-all = まとめて
pi-field-auto-resize = 大きな画像を縮小
pi-field-auto-resize-help = 送信前に最大 2000×2000 に縮小します。
pi-field-block-images = 画像をモデルに送信しない
pi-field-proxy = HTTP プロキシ
pi-field-proxy-help = Pi のリクエストにのみ適用され、Gupi には適用されません。
pi-edit-in-global = グローバルで編集
pi-value-unset = 未設定
pi-field-retry = 自動再試行
pi-field-max-retries = 最大再試行回数
pi-field-base-delay = 初回の待機時間（ミリ秒）
pi-field-max-delay = 最大待機時間（ミリ秒）
pi-source-default = デフォルト
pi-source-global = グローバル
pi-source-project = プロジェクト
pi-unsaved = 未保存
pi-changed-on-disk = 読み込み後にファイル内の値が変更されました。
pi-invalid-count = 0 以上の整数を入力してください。
pi-menu-revert = 変更を取り消す
pi-menu-pin-project = このプロジェクトに設定
pi-menu-pin-global = グローバル設定に設定
pi-menu-inherit = グローバルの値を使用
pi-menu-default = デフォルトを使用
pi-field-actions = 項目の操作
pi-scope-global = グローバル
pi-scope-label = 設定の範囲
pi-trust-untrusted = 未信頼
pi-trust-untrusted-help = プロジェクトを信頼するまで、Pi はこのプロジェクトの設定を使用しません。
pi-trust-action = プロジェクトを信頼…
pi-trust-title = 「{ $name }」を信頼しますか？
pi-trust-body = Pi はこのフォルダーのプロジェクト設定とリソースを読み込み、プロジェクトのパッケージをインストールし、プロジェクトの拡張機能を実行します。親フォルダーは信頼されません。
pi-trust-confirm = プロジェクトを信頼
pi-settings-save = 保存
pi-settings-discard = 破棄
pi-settings-file-actions = その他の操作
pi-settings-open-file = settings.json を開く
pi-settings-reload = 再読み込み
pi-settings-invalid = 保存する前に、示された項目を修正してください。
pi-settings-conflict = 一部の設定は読み込み後にファイル内で変更されました。再読み込みしてから保存してください。
pi-settings-locked = 設定ファイルは別のプロセスが使用中です。もう一度お試しください。
pi-settings-unsaved-title = Pi の設定を保存しますか？
pi-settings-unsaved-body = Pi の設定に未保存の変更があります。
pi-settings-loading = 読み込み中…
pi-settings-unsupported = ここで Pi の設定を編集するには Pi 1.1.0 以降が必要です。現在：{ $version }。
pi-settings-unavailable = Pi を利用できません。「接続」ページを確認してください。
pi-models-loading = モデルを読み込み中…
pi-models-retry = 再試行
pi-project-settings = プロジェクトの Pi 設定…

pi-scope-choose-folder = プロジェクトフォルダを選択…
pi-scope-choose-folder-action = 選択
settings-resource-group-project = このプロジェクト
settings-resource-group-inherited = グローバルから継承
settings-source-project = このプロジェクト
settings-resource-same-global = グローバルと同名 · プロジェクト優先
settings-resource-same-project = プロジェクトと同名 · プロジェクト優先
settings-resource-not-loaded = 未読み込み（プロジェクト未信頼）
settings-resource-edit-global = グローバルで編集
settings-resource-none-project = このプロジェクトの .pi にはまだありません。
settings-system-project-file = プロジェクトのファイル
settings-system-uses-global = グローバルを使用
settings-system-use-global = グローバルを使用
settings-system-use-global-title = { $file } をグローバルに戻しますか？
settings-system-use-global-body = プロジェクトのファイルはゴミ箱に移動し、Pi はグローバルのファイルを使います。
settings-editor-changed = 開いた後にファイルが変更されたため、保存しませんでした。再読み込みしてこの編集を破棄するか、キャンセルしてください。
settings-editor-reload = 再読み込み
settings-editor-unsaved-title = { $file } の変更を保存しますか？
settings-editor-unsaved-body = テキストの変更はまだ保存されていません。

pi-project-settings-folder = プロジェクトの Pi 設定 — { $folder }

packages-install-project = プロジェクト { $folder } にインストール
packages-project-delta = グローバルパッケージのフィルター · 読み取り専用
packages-project-filtered = このプロジェクトでフィルター済み
packages-project-replaced = プロジェクトで置換
packages-project-replaces = グローバルパッケージを置換

settings-resource-disabled = 無効
settings-theme-heading = 独立したテーマ
settings-project-package-empty = このプロジェクトにはパッケージがありません。

pi-model-scope = モデル範囲
pi-model-scope-help = 新しい会話では、既定のモデルが範囲内ならそのモデルを使い、それ以外は最初に一致するモデルを使います。会話中に選択できるモデルは制限しません。
pi-model-scope-add-model = モデルを追加…
pi-model-scope-add-pattern = パターンを追加
pi-model-scope-up = 上へ移動
pi-model-scope-down = 下へ移動
pi-model-scope-delete = パターンを削除
pi-model-scope-unrestricted = 制限なし
pi-model-scope-unrestricted-project = 制限なし（グローバル設定を上書き）
pi-model-scope-blank = 空白のパターンは保存できません。
pi-model-scope-malformed = 文字列の配列である必要があります。設定ファイルで修正してください。

conversation-edited-files = { $count } 個のファイルを編集
conversation-edit-write = 書き込み
conversation-edit-error = エラー
conversation-written-content = 書き込んだ内容
conversation-edit-operation = 操作 { $index }
conversation-edit-more-files = さらに { $count } 個のファイルを表示
conversation-edit-less-files = ファイルを折りたたむ
conversation-edit-more-operations = さらに { $count } 件の操作を表示
conversation-edit-previous-operations = 前の操作
conversation-edit-stats-tooltip = 記録された編集パッチを統合した正味の追加・削除行数です。書き込みと統合できないファイルは除き、作業領域全体の差分ではありません。
conversation-edit-file-label = { $path } — { $state }
conversation-edit-operation-stats-tooltip = この記録されたパッチの追加・削除行数です。
conversation-edit-uncombined = 統合できないため、操作ごとに表示

# Advanced Pi settings
settings-page-pi-tools = ツールと環境
pi-group-tools = ツール
pi-group-shell = シェル
pi-group-package-command = パッケージ管理
pi-group-privacy-trust = プライバシーと信頼
pi-group-advanced = 詳細
pi-group-provider = プロバイダー側の再試行
pi-field-default-tools = 既定のツール
pi-field-codemode-mode = Codemode モード
pi-field-codemode-budget = 宣言の予算
pi-field-shell-path = シェルのパス
pi-field-shell-prefix = コマンドの接頭辞
pi-field-npm-command = npm コマンド
pi-field-telemetry = インストールのテレメトリ
pi-field-project-trust = 既定のプロジェクト信頼
pi-field-model-thinking = モデル別の思考レベル
pi-field-model-overrides = モデル別の圧縮
pi-field-budget-minimal = 最小
pi-field-budget-low = 低
pi-field-budget-medium = 中
pi-field-budget-high = 高
pi-field-cache-warming = キャッシュの事前準備
pi-field-transport = 転送方式
pi-field-http-timeout = HTTP アイドルタイムアウト（ms）
pi-field-websocket-timeout = WebSocket 接続タイムアウト（ms）
pi-field-provider-timeout = リクエストのタイムアウト（ms）
pi-field-provider-retries = 再試行回数
pi-field-provider-delay = 再試行の最大待機時間（ms）
pi-timeout-disabled = 無効
pi-tools-startup-help = 拡張機能と MCP は別のツールも有効にします。セキュリティ制限や実行中の完全なツール一覧ではありません。
pi-map-add-model = モデルを追加
pi-map-remove-local = この範囲の設定を削除
pi-model-thinking-help = 起動時は明示レベルと保存済みセッションが優先され、モデル切り替え時はこの値を使います。モデル範囲の思考サフィックスが優先されます。
pi-model-overrides-help = 各トークン値は個別に継承し、変更した項目だけを保存します。
pi-trust-always-title = プロジェクトを既定で信頼しますか？
pi-trust-always-body = 適用される判断がないプロジェクトを Pi が信頼し、設定を読み込み拡張機能を実行します。既存の明示判断と信頼拡張は引き続き優先されます。
pi-telemetry-env-on = PI_TELEMETRY による上書き：有効
pi-telemetry-env-off = PI_TELEMETRY による上書き：無効
pi-codemode-help = Codemode の設定には、組み込みの codemode 拡張機能と codemode ツールの有効化が必要です。
pi-thinking-budget-help = トークン予算に対応するプロバイダーとモデルにのみ適用されます。
pi-cache-warming-help = 事前準備のリクエストは使用量に計上されます。
pi-telemetry-help = インストールのテレメトリを制御します。更新確認や分析全体の設定ではありません。
pi-provider-retry-help = Pi は 0 のままを推奨します。
pi-trust-always-confirm = 既定で信頼
pi-choice-ask = 確認する
pi-choice-always = 常に信頼
pi-choice-never = 信頼しない
pi-choice-codemode-on = 直接ツールと Codemode
pi-choice-codemode-only = Codemode のみ
pi-choice-streaming = ストリーミング中
pi-choice-idle = 待機中も
pi-choice-auto = 自動

pi-thinking-budgets = 思考予算（トークン）
pi-model-unavailable-short = 利用不可
pi-models-unavailable = モデル一覧を読み込めません
pi-map-model = モデル
pi-map-reserve = 予約トークン
pi-map-keep = 最近の保持トークン
pi-map-token-action = { $field }: { $action }
pi-tools-add = 追加
pi-tools-custom = カスタム…
pi-tool-remove = { $tool } を削除
pi-tools-preview = 起動時: { $tools }
pi-npm-invalid = 引用符を閉じ、最初の引数が空でないコマンドを入力してください。
pi-timeout-off = 無効
pi-npm-help = shell の引用符で空白と空の引数を保持します。
pi-budget-input = { $level }の思考予算（トークン）

pi-group-resource-paths = リソースのパス
pi-field-resource-extensions = 拡張機能
pi-field-resource-skills = スキル
pi-field-resource-prompts = プロンプト
pi-field-resource-themes = テーマ
pi-resource-paths-help = このスコープのファイル内の項目を、そのまま順番に保存します。
pi-resource-unsupported = 非対応の形式 · 読み取り専用
pi-resource-absent = キーなし
pi-resource-empty = 空の配列 []
pi-resource-up = 上に移動
pi-resource-down = 下に移動
pi-resource-remove = 項目を削除
pi-resource-add = 追加
pi-resource-clear = 配列を空にする []
pi-resource-remove-key = キーを削除
pi-resource-entry-actions = 項目の操作

# Pi configuration files
pi-files-models = モデルとプロバイダー
pi-files-mcp = MCP
pi-files-startup-only = Pi は新しいセッションの開始時にこのファイルを読み込みます。既存のセッションは再読み込みされません。
pi-files-global-only = このファイルはグローバル専用です。編集するにはグローバル範囲を選んでください。
pi-files-comments = 保存するとファイル内のコメントが削除されます。
pi-files-add = 追加…
pi-files-edit = 編集…
pi-files-delete = 削除
pi-files-override = 上書き
pi-files-definition = 完全な定義
pi-files-disabled = 無効
pi-files-duplicate = 既存の項目と名前が競合しています。
pi-files-invalid = 項目を読み込みまたは保存できません。フィールドと設定ファイルを確認してください。
pi-files-secret-set = 設定済み · 保持
pi-files-secret-keep = 保持
pi-files-secret-replace = 新しい値に置換
pi-files-secret-remove = 除去
pi-files-name = 名前
pi-files-key = キー
pi-files-model = モデル
pi-files-add-model = モデルを追加
pi-files-on = オン
pi-files-off = オフ

# Structured configuration editor
pi-files-label-name = 表示名
pi-files-label-id = モデル ID
pi-files-label-baseUrl = ベース URL
pi-files-label-api = API
pi-files-label-apiKey = API キー
pi-files-label-authHeader = 認証ヘッダー
pi-files-label-oauth = OAuth
pi-files-label-compat = 互換性
pi-files-label-models = モデル
pi-files-label-modelOverrides = モデルの上書き
pi-files-label-enabled = 有効
pi-files-label-exposure = 公開方法
pi-files-label-type = 通信方式
pi-files-label-description = 説明
pi-files-label-command = コマンド
pi-files-label-url = URL
pi-files-label-cwd = 作業ディレクトリ
pi-files-label-timeout = タイムアウト
pi-files-label-toolExposure = ツールの公開方法
pi-files-label-args = 引数
pi-files-label-env = 環境変数
pi-files-label-headers = ヘッダー
pi-files-label-clientId = クライアント ID
pi-files-label-clientSecret = クライアントシークレット
pi-files-label-callbackPort = コールバックポート
pi-files-label-callbackUrl = コールバック URL
pi-files-label-scope = スコープ
pi-files-label-clientName = クライアント名
pi-files-label-clientRegistration = クライアント登録
pi-files-label-authServerMetadataUrl = 認証メタデータ URL
pi-files-label-provider = プロバイダー
pi-files-label-reasoning = 推論
pi-files-label-contextWindow = コンテキスト長
pi-files-label-maxTokens = 最大出力トークン
pi-files-label-input = 入力形式
pi-files-label-cost = 料金
pi-files-label-thinkingLevelMap = 思考レベルの対応
pi-files-label-inputLimits = 入力制限
pi-files-label-promptCache = プロンプトキャッシュ
pi-files-label-samplingParams = サンプリング設定
pi-files-label-samplingParamsByThinkingLevel = 思考レベル別サンプリング
pi-files-label-autoEnableCodemode = Codemode を自動有効化
pi-files-providers = プロバイダー
pi-files-servers = MCP サーバー
pi-files-automation = Codemode
pi-files-empty = 項目はありません
pi-files-basic = 基本
pi-files-connection = 接続
pi-files-exposure-section = 公開方法
pi-files-authentication = 認証
pi-files-advanced = 詳細
pi-files-back = 戻る
pi-files-stage = 変更を一時保存
pi-files-secret-present = 設定済み
pi-files-add-value = 新しい値
pi-files-confirm-add = 追加
pi-files-definition-kind = 定義
pi-files-provider-summary = モデル { $models } 件 · 上書き { $overrides } 件

# Pi provider authentication
pi-auth-title = 認証
pi-auth-help = 認証情報は Pi の auth.json に保存されます。変更は新しいセッションで完全に反映されます。Gupi は既存セッションの再接続やモデル一覧の同期を行いません。
pi-auth-add = API キーを追加…
pi-auth-replace = API キーを置換…
pi-auth-api-key = API キー
pi-auth-oauth = OAuth
pi-auth-unknown = 不明な形式
pi-auth-missing = 保存済みの認証情報なし
pi-auth-set = 設定済み
pi-auth-empty = プロバイダーがありません。API キーを追加してください。
pi-auth-provider = プロバイダー ID
pi-auth-key-placeholder = 新しい API キーを入力
pi-auth-saving = 保存中…
pi-auth-candidates = プロバイダーを選択
pi-auth-check = 設定を確認
pi-auth-checking = 設定を確認中
pi-auth-ready = Pi は使用可能な認証設定を検出しました。トークンの有効性は検証していません。
pi-auth-not-ready = Pi は使用可能な認証設定を検出できませんでした。
pi-auth-invalid = Pi が無効な設定を報告しました。
pi-auth-check-failed = 設定を確認できませんでした。
pi-auth-provider-not-found = プロバイダーが見つかりません。
pi-auth-not-configured = 認証情報が設定されていません。
pi-auth-not-available = 認証情報を利用できません。
pi-auth-invalid-state = Pi が認証状態を読み取れませんでした。
pi-auth-copy-login = ログインコマンドをコピー
pi-auth-login-help = コマンドをコピーしました。Pi のターミナルで実行して再ログインしてください。
pi-auth-oauth-help = ローカルの記録のみを表示します。Pi は使用時に認証情報の更新を試みる場合があります。必要に応じて再ログインしてください。
pi-auth-expired = 記録された有効期限を超過
pi-auth-expires = 記録された有効期限：{ $time }
pi-auth-remove = 認証情報を削除…
pi-auth-logout = ログアウト…
pi-auth-remove-title = 「{ $provider }」の認証情報を削除しますか？
pi-auth-logout-title = 「{ $provider }」からログアウトしますか？
pi-auth-remove-help = ローカルの認証情報のみを削除します。サーバーのトークンは失効しません。環境変数や models.json の apiKey は引き続き有効な場合があります。
pi-auth-remove-env = プロバイダーの環境値と設定値も削除されます。
pi-auth-remove-confirm = 削除
pi-auth-logout-confirm = ログアウト
pi-auth-replace-title = 「{ $provider }」の認証情報を置換しますか？
pi-auth-replace-help = トークンや不明なフィールドを含む認証情報全体を API キーに置換します。
pi-auth-replace-confirm = 置換
pi-auth-read-error = auth.json を読み取れませんでした。
pi-auth-parse-error = auth.json は有効な JSON オブジェクトである必要があります。編集は無効です。
pi-auth-busy = auth.json は使用中です。再試行してください。
pi-auth-conflict = このプロバイダーはディスク上で変更されました。再読み込みして再試行してください。
pi-auth-write-error = auth.json に書き込めませんでした。再読み込みして現在の状態を確認してください。
pi-auth-input-error = プロバイダーを選ぶか、空でない単一行の ID と API キーを入力してください。
pi-auth-login-select-help = コマンドをコピーしました。Pi のターミナルで /login を実行し、プロバイダーを選択してください。

# Authentication and model provider settings
pi-files-auth-stored = 認証ページに認証情報あり
pi-files-manage-auth = 認証ページで管理
pi-files-api-key-auth-help = 認証ページに認証情報が保存されている場合、Pi はこの apiKey にフォールバックしません。保存済みでも認証が有効とは限りません。
pi-auth-models-help = 保存済みの認証情報は models.json の apiKey の使用に影響します。実際の認証は Pi がプロバイダーの規則に従って処理します。
pi-auth-edit-models = モデルとプロバイダーで編集

pi-provider-empty-id = 〈空の ID〉
pi-auth-check-unavailable = Pi はこのプロバイダー ID を確認できません。

settings-skill-name-help = ディレクトリ名。Pi の推奨は小文字、数字、単独のハイフンで、64 文字以内です。

settings-skill-name-warning = Pi の推奨形式ではありませんが、作成できます。SKILL.md 内の name はここでは検証しません。

settings-prompt-name-help = /名前 で呼び出します。空白は使用できません。

settings-prompt-invalid-name = 空白やパス区切り文字を含まない有効なファイル名を入力してください。

shortcut-add-title = テンプレートタスクを追加

shortcut-edit-title = テンプレートタスクを編集

shortcut-options-failed = テンプレートまたはモデルの選択肢を読み込めませんでした。Pi の接続を確認して再試行してください。

pi-auth-row-action = { $provider }: { $action }

pi-auth-add-title = API キーを追加

pi-auth-replace-editor-title = API キーを置換

temporary-clean-title = 解放済みの作業ディレクトリを整理しますか？

settings-package-remove-title = 「{ $name }」を削除しますか？

settings-resource-delete-title = 「{ $name }」をゴミ箱に移動しますか？

shortcut-delete-title = テンプレートタスク「{ $name }」を削除しますか？

settings-key-control-label = { $command }: { $binding }
settings-key-control-recording = { $command }: { $binding } — { $state }

# MCP CLI login and last-check results
pi-mcp-help = Pi 組み込み MCP の確認とログインです。代替拡張機能のセッション状態は異なる場合があります。auth.provider を使うサービスは、ここではログインが必要でもセッション内では利用できる場合があります。
pi-mcp-reconnect = 実行中のセッションに保存済み MCP 設定を完全に読み込むには、会話メニューの「再接続」を選んでください。新しいセッションには自動で読み込まれます。
pi-mcp-global-cwd = グローバル確認は空の一時フォルダーで実行するため、相対パスの動作がセッション内と異なる場合があります。
pi-mcp-save-first = 先に変更を保存または破棄してください
pi-mcp-check = 接続を確認
pi-mcp-running = 実行中…
pi-mcp-last-check = 前回の確認 { $time }
pi-mcp-config-error = 設定にエラーがあります
pi-mcp-untrusted = 未信頼のプロジェクトのため、Pi はグローバル設定のみ使用します
pi-mcp-resources = リソースあり
pi-mcp-connected = 接続済み（ツール { $count } 個）
pi-mcp-needs-auth = ログインが必要
pi-mcp-connection-failed = 接続に失敗
pi-mcp-disconnected = 未接続
pi-mcp-not-checked = 未確認
pi-mcp-unsupported-name = この名前では Pi CLI のログイン・ログアウトを利用できません
pi-mcp-login = ログイン
pi-mcp-logout = ログアウト
pi-mcp-login-title = MCP サービス「{ $name }」にログイン
pi-mcp-logout-title = MCP サービス「{ $name }」からログアウトしますか？
pi-mcp-login-help = 必要に応じて Pi がブラウザーを開きます。ブラウザーでログインを完了してください。
pi-mcp-logout-help = ローカルの認証情報を削除します。サーバー側の許可は取り消されず、実行中のセッションも直ちには切断されません。接続を更新するには再接続してください。
pi-mcp-login-terminal = ブラウザーが開かない場合はキャンセルし、ターミナルで pi mcp login -- { $name } を実行してください。
pi-mcp-completed = コマンド完了
pi-mcp-cancelled = キャンセルしました
pi-mcp-timeout = 時間切れのため結果は不完全です
pi-mcp-login-failed = ログインに失敗しました。認証情報の保存後に接続が失敗した可能性があります。接続を確認してください。
pi-mcp-failed = 操作に失敗しました
pi-mcp-close = 閉じる
pi-mcp-scope-project = プロジェクト
pi-mcp-disabled = 無効
pi-mcp-check-failed = 確認に失敗しました
pi-mcp-check-timeout = 確認が時間切れのため結果は不完全です

workspace-changes = 変更
workspace-close-tab = タブを閉じる
workspace-close-others = 他のタブを閉じる
workspace-close-right = 右側のタブを閉じる
workspace-toggle-file-tree = ファイルツリーの表示切替
workspace-toggle-change-list = 変更ファイル一覧の表示切替
workspace-out-of-scope = この返信は現在の範囲外です。
workspace-net-zero = この返信の変更はすべて相殺されました。
workspace-open-file = ファイルを開く
workspace-next-operations = 次の操作
workspace-open-change = ワークスペースで { $path } を開く

history-panel-title = 履歴
history-panel-options = 履歴のオプション
workspace-view-changes = 変更を表示

workspace-new-tab = 新しいタブ
workspace-all-tabs = すべてのタブ
workspace-tools = ツール
workspace-recent = 最近使った項目
workspace-no-changes = この会話にはまだ変更がありません。
workspace-recent-unavailable = 現在の範囲ではこの変更を開けません。
workspace-just-now = たった今
workspace-minutes-ago = { $count } 分前
workspace-maximize = ワークスペースを最大化
workspace-restore = ワークスペースを元に戻す

# Direct Shell command output
shell-title = Shell
shell-running = コマンドを実行中
shell-stopping = コマンドを停止中
shell-stop = コマンドを停止
shell-cancelled = キャンセル済み
shell-exit-code = 終了コード { $code }
shell-completed = 完了 · 終了コードなし
shell-excluded = モデルのコンテキストに含めない
shell-show-all = 出力をすべて表示
shell-show-less = 出力を折りたたむ
shell-open-full-output = 完全な出力を開く
shell-display-truncated = 最新の出力のみ表示しています。

titlebar-git-branch = Git ブランチ: { $name }
titlebar-git-detached = 切り離された HEAD: { $name }

# Diff reader
diff-worktree = Git
diff-unstaged = 未ステージ
diff-staged = ステージ済み
diff-untracked = 未追跡
diff-filter = パスを絞り込み…
diff-previous-file = 前のファイル
diff-next-file = 次のファイル
diff-unified = 統合
diff-split = 左右比較
diff-wrap = 折り返し
diff-actions = ファイル操作
diff-word = 単語内の差分
diff-context = 変更のないすべての行
diff-expand-file = このファイルを展開
diff-copy-path = ファイルパスをコピー
diff-copy-patch = パッチをコピー
diff-copy-content = ファイル全文をコピー
diff-external = 外部エディターで開く
diff-refresh = 変更を更新
diff-refresh-failed = 変更を更新できません。表示結果が古い可能性があります。
diff-loading = 変更を読み込み中…
diff-not-repository = このフォルダーは Git 作業ツリーにありません
diff-empty = 変更なし
diff-no-results = 一致するファイルなし
diff-retry = 再試行
diff-recorded-context = この実行の変更 · 記録されたコンテキストのみ
diff-added = 追加
diff-deleted = 削除
diff-modified = 変更
diff-conflict = 競合 · 読み取り専用
diff-special = 特殊ファイル · 読み取り専用
diff-binary = バイナリファイル
diff-too-large = テキスト表示の制限を超えています
diff-unavailable-file = 現在のファイルを利用できないため、外部で開けません。
diff-git-unavailable = Git を利用できません。Git をインストールして更新してください。
diff-files = ファイル
diff-conflicts = 競合
diff-direction-staged = HEAD → インデックス
diff-direction-unstaged = インデックス → 作業ツリー
diff-direction-untracked = 空 → 作業ツリー
diff-updating = 以前の版を更新中…
diff-collapse-file = このファイルを折りたたむ
diff-update-failed = この版を更新できませんでした
diff-reading-options = 表示オプション
diff-reading-options-narrow = 表示オプション：本文領域が狭いため左右表示できません。広げると設定した表示に戻ります
diff-view = ビュー
diff-display = 表示
diff-list-view = リスト
diff-tree-view = ツリー
diff-stats-incomplete = 統計は不完全です

workspace-back-conversation = 会話に戻る
workspace-back-content = 本文に戻る
workspace-open = ワークスペースを開く
workspace-close = ワークスペースを閉じる

diff-invalid-path = ファイル名が有効な UTF-8 ではないため読み取れません
diff-no-text-changes = テキストの変更はありません

resource-open-failed = ファイルを開けませんでした。ファイルが存在しないか、対応するアプリがありません。

diff-conflict-label = 競合
diff-read-failed = 変更を読み込めませんでした。再試行してください。

# Conversation navigation
conversation-navigation-jump = ユーザーメッセージ { $number } に移動：{ $question }
conversation-navigation-current = 現在の区間。ユーザーメッセージ { $number } に移動：{ $question }
conversation-navigation-no-text = テキストなし

error-technical-details = 技術的な詳細
conversation-load-failed = 会話の内容を読み込めませんでした。
conversation-refresh-failed = 会話の内容を更新できませんでした。表示中の内容は最新でない可能性があります。
history-load-failed = 会話履歴を読み込めませんでした。
history-refresh-failed = 会話履歴を更新できませんでした。表示中の履歴は最新でない可能性があります。
conversation-runtime-failed = Pi の実行中に問題が発生しました。
conversation-storage-failed = ローカルの会話データにアクセスできませんでした。詳細を確認し、ファイルの場所と権限を確認してください。
composer-model-load-failed = モデル一覧を読み込めませんでした。
composer-thinking-load-failed = 思考レベルの選択肢を読み込めませんでした。
composer-model-change-failed = モデルまたは思考レベルの変更結果を確認できませんでした。
command-load-failed = コマンド一覧を読み込めませんでした。
resource-files-failed = ファイルを検索できませんでした。パスを確認して再度検索してください。

quote-in-composer = 入力欄に引用
quote-select-text = 引用するテキストを選択してください。
quote-select-diff-lines = 差分の片側の行番号から選択してください。
quote-composer-unavailable = 現在、通常のメッセージ下書きは編集できません。
quote-message-title =
    { $role ->
        [user] ユーザー — 選択時の内容：
       *[assistant] アシスタント — 選択時の内容：
    }
quote-file-title = ファイル — 選択時の内容：{ $path }、{ $start }–{ $end } 行
quote-turn-title = 今回の変更 — 選択時の内容：{ $path }、{ $side ->
        [original] 変更前
       *[modified] 変更後
    }、{ $start }–{ $end } 行
quote-staged-title = Git · ステージ済み — 選択時の内容：{ $path }、{ $side ->
        [original] 変更前
       *[modified] 変更後
    }、{ $start }–{ $end } 行
quote-unstaged-title = Git · 未ステージ — 選択時の内容：{ $path }、{ $side ->
        [original] 変更前
       *[modified] 変更後
    }、{ $start }–{ $end } 行
quote-untracked-title = Git · 未追跡 — 選択時の内容：{ $path }、{ $side ->
        [original] 変更前
       *[modified] 変更後
    }、{ $start }–{ $end } 行
quote-disabled-menu =
    { $reason ->
        [selection] 入力欄に引用 — テキストを選択してください
        [diff] 入力欄に引用 — 片側の行番号を選択してください
       *[composer] 入力欄に引用 — 下書きを編集できません
    }

quote-action = 引用
quote-comment-placeholder = コメントを追加（任意）…
quote-comment-prefix = { "コメント：" }
quote-message-short =
    { $role ->
        [user] あなたのメッセージ
       *[assistant] アシスタントの回答
    }
quote-file-short = { $name } · { $start }–{ $end } 行
quote-diff-short = { $name } · { $side ->
        [original] 変更前
       *[modified] 変更後
    } · { $start }–{ $end } 行
quote-command-unavailable = このコマンドにはまだ引用を添付できません。通常のメッセージに書き直すか、引用を削除してから送信してください。

quote-count =
    { $count ->
       *[other] { $count } 件の引用
    }
quote-selected-text = 選択したテキスト：
quote-list-source = { $source }：
quote-user-comment = ユーザーのコメント：
quote-edit-comment = コメントを編集
quote-delete-number = 引用 { $number } を削除：{ $source }
quote-clear = 引用をすべて削除
quote-edit-label = コメント { $number } を編集：{ $source }
files-unavailable = プレビューできません
files-open-external = デフォルトのアプリで開く
files-open-remote-image = ブラウザーで画像を開く
image-preview-fit = 全体を表示
image-preview-actual-size = 実際のサイズ
