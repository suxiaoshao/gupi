app-title = Gupi
menu-settings = 設定
menu-show-main = 主視窗
menu-quit = 退出 Gupi
startup-welcome = 歡迎使用 Gupi
temporary-setup-required = 請在設定中完成設定，再開始臨時對話。
startup-checking = 正在檢查…
startup-quitting = 正在完成操作…
settings-pi-command = Pi 可執行檔
settings-path-help = 留空以自動尋找 pi，或填寫可執行檔的絕對路徑，不包含參數。
settings-theme = 外觀
settings-language = 語言
settings-save-pi = 儲存 Pi 路徑
settings-reload = 從磁碟重讀
settings-write-current = 將已套用設定寫回磁碟（保留草稿）
theme-system = 跟隨系統
theme-light = 淺色
theme-dark = 深色
language-system = 跟隨系統
language-english = English
language-chinese = 簡體中文
recovery-config-title = 設定需要處理
recovery-startup-title = Gupi 無法啟動
error-instance-startup = 請檢查設定資料夾權限，或關閉其他正在執行的 Gupi 後重試。
recovery-pi-title = 設定 Pi 環境
recovery-confirm = 確定繼續？重讀成功後才會捨棄草稿；重置會先備份當前檔案。
recovery-backup = 備份位置
error-config-read = 無法讀取設定，請檢查檔案位置和權限後重讀。
error-config-parse = 設定格式無效。可修正檔案後重讀，或備份並重置。
error-config-validation = 請填寫單個可執行檔名或絕對可執行路徑，不附帶參數。
error-config-write = 儲存失敗，草稿與已套用設定已保留。請檢查設定檔後重試。
error-pi-probe = 無法驗證 Pi。請檢查可執行檔路徑和 Node 環境後重試。
action-check-pi = 重新檢查 Pi
action-reset = 備份並重置
action-confirm = 確認
action-cancel = 取消
home-ready = Pi 已就緒
home-description = 當前版本提供環境設定與桌面偏好。對話功能將在下一階段接入。
action-locate = 顯示設定檔案資料夾
error-pi-timeout = Pi 未在 15 秒內完成版本檢查。
error-pi-output = Pi 返回的內容超出了版本檢查的輸出限制。
error-pi-version = Pi 未正常退出，或未返回有效版本號。
error-log = 無法寫入日誌檔案，請檢查日誌目錄與權限。

setup-tagline = 為 Pi 準備的原生桌面空間
setup-intro = 簡單設定，即可開始。
setup-start = 開始設定
setup-language-title = 選擇你的語言
setup-search-language = 搜尋語言…
setup-system-language = 系統語言：{ $language }
setup-appearance-title = 選擇喜歡的外觀
setup-color-mode = 亮暗模式
setup-system-accent = 系統強調色
setup-pi-title = 連接本機 Pi
setup-pi-step = Pi 設定
setup-pi-help = 可在此設定並檢測本機 Pi，也可以跳過此頁，稍後再設定。
setup-check-pi = 檢測 Pi
setup-back = 上一步
setup-next = 下一步
setup-finish = 完成設定
setup-saving = 正在儲存…
light-themes = 淺色主題
dark-themes = 深色主題

# Conversation workspace
conversation-new = 新建對話
conversation-sidebar = 對話側邊欄
conversation-history = 對話歷史
conversation-restoring = 正在恢復草稿…
conversation-discovering = 正在尋找對話… 已發現 { $count } 個檔案
conversation-read-progress = 已讀取 { $completed } / { $total } 個檔案
conversation-refresh-progress = 正在刷新 { $completed } / { $total } 個檔案
conversation-project = 選擇工作目錄
conversation-project-search-placeholder = 搜尋專案
conversation-project-search-empty = 沒有相符的專案
conversation-open-folder = 開啟資料夾…
conversation-untitled = 未命名對話
conversation-idle = 空閒
conversation-loading = 正在打開
conversation-running = 正在執行
conversation-failed = 需要處理錯誤
conversation-waiting = 等待輸入
conversation-search = 搜尋對話
conversation-refresh = 刷新對話目錄
conversation-refresh-current = 刷新當前對話
conversation-actions = 對話操作
conversation-show-sidebar = 展開對話側欄
conversation-hide-sidebar = 收起對話側欄
conversation-scan-failed = 對話目錄加載失敗
conversation-search-placeholder = 搜尋名稱、消息或項目路徑
conversation-search-empty = 沒有匹配的對話
conversation-rename = 重命名…
conversation-delete = 移到廢紙簍
conversation-delete-failed = 無法刪除對話：{ $error }
conversation-copy-path = 複製路徑
conversation-clone = 複製對話
conversation-export = 導出對話為 HTML…
conversation-exported = 對話已導出至 { $path }
conversation-stop = 停止生成
conversation-close-run = 結束執行
conversation-show-less = 收起
conversation-show-more = 查看更多…
conversation-continue = 從此處繼續
conversation-fork = 從這裡另開對話
conversation-preview = 正在預覽其他分支
conversation-return-current = 返回當前分支
conversation-welcome = 開始一段對話
conversation-welcome-project = 你想在 { $project } 中做些什麼？
conversation-empty-history = 此對話還沒有消息。
conversation-bottom = 返回底部
conversation-copy = 複製
conversation-compaction = 上下文壓縮摘要
conversation-branch-summary = 分支摘要
conversation-working = 正在處理…
conversation-process = 查看處理過程
conversation-details = 展開或收起細節
tool-detail-offset = 起始行：
tool-detail-line-limit = 請求的行數上限：
tool-detail-timeout = 超時（秒）：
tool-detail-output = 輸出
tool-detail-input = 輸入
tool-detail-additional = 附加資訊
tool-detail-error = 錯誤
tool-detail-glob = 檔案匹配模式：
tool-detail-ignore-case = 忽略大小寫：
tool-detail-literal = 按字面文本搜尋：
tool-detail-context = 上下文行數：
tool-detail-result-limit = 請求的結果數量上限：
tool-detail-old-text = 請求匹配的原文
tool-detail-new-text = 請求替換的內容
tool-detail-truncated = Pi 已截斷輸出，當前顯示的內容並非完整結果。
tool-detail-lines-truncated = Pi 已縮短部分結果行。
tool-detail-match-limit = 已達到匹配數量上限：
tool-detail-results-limited = 已達到結果數量上限：
tool-detail-entries-limited = 已達到目錄條目數量上限：
tool-detail-full-output = 完整輸出檔案：
tool-detail-image-unavailable = 無法顯示此圖片。
conversation-role-user = 使用者消息
conversation-role-assistant = 助手消息
conversation-role-tool = 工具結果
conversation-event = 對話事件
conversation-close-history = 收起對話歷史
conversation-source = 查看來源對話檔案
conversation-reconnect = 重新連接
conversation-save-error = 草稿儲存失敗
conversation-interrupted = 本次執行已停止；已保留現有內容。
conversation-no = 否
conversation-submit = 提交
conversation-input = 輸入消息
conversation-model = 模型
conversation-load-options = 連接 Pi 以加載可用模型
conversation-thinking = 思考強度
conversation-unknown = 暫不可用
conversation-context = 當前執行分支上下文
conversation-auto-compaction = 自動壓縮
conversation-on = 已開啓
conversation-off = 已關閉
conversation-tokens = 累計輸入 / 輸出
conversation-cache = 累計緩存讀取 / 寫入
conversation-cache-hit = 最近緩存命中率
conversation-cost = Pi 統計費用
conversation-statistics = 用量統計
conversation-send = 發送（Enter）；Alt+Enter 等待本輪結束後發送
conversation-sending = 正在發送…
conversation-graph-current = 當前執行
conversation-graph-preview = 預覽
conversation-graph-left = 查看左側軌道（也可橫向滾動）
conversation-graph-right = 查看右側軌道（也可橫向滾動）
conversation-graph-reveal = 定位選中
conversation-graph-column = 分支
conversation-graph-message = 消息
conversation-catalog-empty = 暫無對話

conversation-thinking-content = 思考過程
conversation-thinking-running = 正在思考
conversation-tool-group = 工具調用（{ $count }）
conversation-tool-group-read = 讀取檔案（{ $count }）
conversation-tool-group-bash = 執行指令（{ $count }）
conversation-tool-group-search = 尋找內容（{ $count }）
conversation-tool-group-edit = 修改檔案（{ $count }）
conversation-tool-line = { $action } { $summary }
conversation-tool-action-read =
    { $state ->
        [running] 正在讀取
        [complete] 已讀取
        [failed] 讀取失敗
        *[unfinished] 未完成讀取
    }
conversation-tool-action-write =
    { $state ->
        [running] 正在寫入
        [complete] 已寫入
        [failed] 寫入失敗
        *[unfinished] 未完成寫入
    }
conversation-tool-action-edit =
    { $state ->
        [running] 正在編輯
        [complete] 已編輯
        [failed] 編輯失敗
        *[unfinished] 未完成編輯
    }
conversation-tool-action-bash =
    { $state ->
        [running] 正在執行
        [complete] 已執行
        [failed] 執行失敗
        *[unfinished] 未完成執行
    }
conversation-tool-action-search =
    { $state ->
        [running] 正在尋找
        [complete] 已尋找
        [failed] 尋找失敗
        *[unfinished] 未完成尋找
    }
conversation-tool-action-other =
    { $state ->
        [running] 正在調用 { $name }
        [complete] 已調用 { $name }
        [failed] 調用失敗 { $name }
        *[unfinished] 未完成調用 { $name }
    }

conversation-processed = 已處理 { $duration }
conversation-processed-failed = 處理失敗 { $duration }
conversation-processed-stopped = 已停止 { $duration }
conversation-copied = 已複製
conversation-copy-failed = 複製失敗，請重試
conversation-usage-title = 本次請求用量
conversation-usage-model = 模型
conversation-usage-provider = 服務商
conversation-usage-input = 輸入 Token
conversation-usage-output = 輸出 Token
conversation-usage-cache-read = 緩存讀取 Token
conversation-usage-cache-write = 緩存寫入 Token
conversation-usage-total = 總 Token
conversation-usage-cost = 費用

composer-context-used = 已佔用 Token
composer-context-limit = 上下文容量
composer-context-percent = 佔用比例
composer-token-input = 累計輸入 Token
composer-token-output = 累計輸出 Token
composer-token-cache-read = 累計緩存讀取 Token
composer-token-cache-write = 累計緩存寫入 Token
composer-token-usage = 對話 Token 用量

conversation-model-search = 搜尋模型…
conversation-model-empty = 沒有匹配的模型
conversation-thinking-off = 關閉
conversation-thinking-minimal = 最小
conversation-thinking-low = 低
conversation-thinking-medium = 中
conversation-thinking-high = 高
conversation-thinking-xhigh = 超高
conversation-thinking-max = 最大

conversation-model-reasoning = 推理
conversation-model-vision = 視覺

conversation-history-reply = 助手回復
conversation-history-brief = 簡略：使用者消息與最終回答
conversation-history-detailed = 詳細：消息、工具和摘要
conversation-history-progress = 助手過程
conversation-history-thinking = 思考
conversation-history-calls = 工具調用
conversation-history-failed = 執行失敗
conversation-history-stopped = 已中斷
conversation-history-empty = 無正文回復
composer-model-thinking = 模型與思考程度
composer-model-refresh = 刷新模型與思考檔位
composer-thinking-unavailable = 不支持思考調節
composer-model-loading = 正在讀取模型列表…
composer-thinking-loading = 正在讀取思考檔位…
composer-model-confirming = 正在確認模型設定…
composer-stats-loading = 正在讀取用量…
composer-stats-retry = 重新讀取用量
conversation-checking-file = 正在檢查對話檔案…
conversation-connecting = 正在連接 Pi…
conversation-history-loading = 正在讀取對話歷史…
conversation-history-refreshing = 正在刷新對話歷史…
conversation-fork-options-loading = 正在讀取另開對話選項…
conversation-fork-options-retry = 重新讀取另開對話選項
history-canvas-zoom-in = 放大
history-canvas-zoom-out = 縮小
history-canvas-fit = 查看全圖（0）
history-canvas-current = 定位當前執行節點
history-canvas-expand = 展開 { $count } 個節點
history-canvas-collapse = 收起 { $count } 個過程節點，保留當前定位
history-canvas-empty = 暫無可顯示的歷史節點
history-canvas-help = 對話樹畫布：滾動或拖動平移，捏合或加減號縮放，方向鍵選擇，Enter 預覽，E 展開下一段

history-view-tree = 樹
history-view-list = 列表
history-level-brief = 簡略
history-level-detailed = 詳細
history-level-all = 全部
history-level-all-description = 全部：包括設定變化、標籤和自定義記錄
history-scope-all = 全部分支
history-scope-branch = 僅此分支
history-model-change = 模型變更
history-thinking-change = 思考級別變更
history-session-info = 對話資訊變更
history-label-change = 標籤變更
history-custom-record = 自定義記錄


history-content = 內容
history-range = 範圍

command-palette = 指令面板
command-focus-input = 聚焦對話輸入
command-model = 選擇模型與思考等級…
command-copy-last-answer = 複製最後回答
command-compact = 壓縮上下文
conversation-compacting = 正在壓縮上下文…
command-history-description = 查看歷史，或選擇使用者消息分叉
command-show-history = 顯示歷史
command-hide-history = 隱藏歷史
command-current-session = 當前對話：
command-unavailable = 當前狀態不可用
command-target-changed = 此對話已不可用，請選擇其他對話。
command-scanning = 正在加載對話…
command-loading = 正在加載 Pi 指令…
command-no-matches = 沒有匹配的 Pi 指令
conversation-reconnect-unconfirmed = 無法確認 Pi 進程已退出，請重啓 Gupi 後再連接。
action-retry = 重試

command-search-placeholder = 搜尋操作或輸入指令
command-empty = 當前對話沒有 Pi 指令

command-group-app = 應用程式
command-group-extensions = 外掛指令
command-group-skills = 技能
command-group-prompts = 提示詞範本
command-scope-user = 個人
command-scope-project = 項目
command-scope-temporary = 臨時
command-connection-unavailable = Pi 尚未就緒
command-dismiss = 退出
command-complete = 補全
command-send-text = 發送
command-execute = 執行

command-scope-current = 當前對話

conversation-working-duration = 正在處理 { $duration }
conversation-tool-group-skill = 讀取技能（{ $count }）
conversation-shell-line = { $shell } · { $action } { $summary }
conversation-tool-action-skill =
    { $state ->
        [running] 正在讀取技能
        [complete] 已讀取技能
        [failed] 技能讀取失敗
       *[unfinished] 技能讀取未完成
    }

# Unified settings
settings-page-general = 通用
settings-page-pi = Pi
settings-page-keys = 快速鍵
settings-page-skills = Skill
settings-page-prompts = 提示詞
settings-page-about = 關於
settings-key-reset = 恢復預設
settings-key-reset-all = 全部恢復預設
settings-key-reset-all-confirm = 恢復所有快速鍵的預設綁定？所有自定義快速鍵將被清除。
settings-key-clear = 清除快速鍵
settings-key-cancel = 取消本次修改
settings-key-unbound = 未設定快速鍵
settings-key-recording = 按下快速鍵，Esc 取消
settings-key-conflict = 快速鍵衝突或無效
settings-key-invalid = 請輸入有效的快速鍵；留空表示取消綁定
settings-resource-refresh = 重新讀取資源
settings-resource-reload-help = 僅管理個人資源。修改後，請手動刷新需要更新的對話。
settings-resource-working = 正在處理…
settings-resource-pi-required = 請先在 Pi 設定中儲存並檢查可用的 Pi 指令。
settings-resource-invalid-name = 名稱只能包含英文字母、數字、連字符或下划線。
settings-resource-register = 添加本地
settings-resource-create = 創建
settings-resource-name-help = 新資源名稱：英文字母、數字、連字符或下划線。
settings-resource-empty = 沒有匹配的資源。
settings-resource-delete = 移到廢紙簍
settings-resource-save = 儲存正文
shortcut-save-task = 儲存
settings-resource-loading = 正在讀取正文…
settings-resource-confirm-remove = 確認移除此項？刪除檔案僅將所選檔案移到廢紙簍，不會刪除整個目錄。
settings-package-install = 安裝包
settings-package-update = 更新
settings-package-remove = 移除
settings-package-source-help = 包來源：npm:包名、Git 地址或本地絕對路徑。
settings-package-extensions = 擴展 { $count }
settings-package-skills = Skill { $count }
settings-package-prompts = 範本 { $count }
settings-package-themes = 主題 { $count }
settings-skill-search = 搜尋 Skill 名稱、說明、路徑或來源
settings-skill-collapse = 收起
settings-resource-edit = 編輯
settings-about-gupi = 版本
settings-about-pi = 版本
settings-about-path = 實際路徑
settings-about-status = 檢查狀態
settings-about-unavailable = 尚未取得
settings-editor-discard = 放棄未儲存的正文修改嗎？
settings-package-confirm-remove = 從所選 Pi 範圍中移除此套件？本機套件只取消註冊，保留來源目錄。
settings-package-heading = 已安裝包
settings-extension-heading = 獨立擴展
settings-template-heading = 指令範本
settings-system-heading = 系統提示詞
settings-config-heading = 設定檔
settings-config-open = 打開設定檔
settings-mode-help = 跟隨系統時，自動使用對應的淺色或深色主題。
settings-light-help = 淺色模式下使用此主題。
settings-dark-help = 深色模式下使用此主題。
settings-pi-unsaved = 路徑尚未儲存；檢查僅驗證當前輸入，不會儲存修改。
settings-pi-saved = 當前路徑已儲存。檢查可重新確認 Pi 是否可用。
settings-resource-location = 顯示位置
settings-package-source = 包來源
settings-package-empty = 尚未安裝個人 Pi 包。
settings-resource-name = 名稱
settings-resource-none = 尚無此類個人資源。
settings-resource-view = 查看
settings-resource-readonly-badge = 只讀
settings-source-personal = 個人
settings-source-external = 外部來源
settings-system-replace = 替換預設系統提示詞
settings-system-replace-help = 完整替換 Pi 的預設系統提示詞。
settings-system-append = 追加個人指令
settings-system-append-help = 保留預設系統提示詞，並在末尾追加這些指令。
settings-resource-success = 已完成對“{ $target }”的操作。已有對話需手動刷新。
settings-resource-failed = 對“{ $target }”的操作失敗。
settings-about-help = 顯示已儲存路徑最近一次檢查的結果；如需重新檢查，請進入 Pi 設定。
settings-key-reveal-session = 顯示對話檔案

settings-key-group-app = 應用程式
settings-key-group-conversation = 對話
settings-key-group-files = 對話檔案

settings-package-kind-extensions = 擴展
settings-package-kind-skills = Skill
settings-package-kind-themes = 主題
settings-package-kind-prompts = 範本
settings-package-expand = 展開包內容
settings-package-collapse = 收起包內容

settings-template-search = 搜尋範本名稱、說明或來源
settings-template-collapse = 收起預覽

temporary-title = 臨時對話
attachment-add = 添加附件
attachment-remove = 移除附件
attachment-file = 檔案
attachment-clipboard = 剪貼簿圖片
image-preview-close = 關閉預覽
image-preview-open = 查看圖片
image-preview-zoom-in = 放大
image-preview-zoom-out = 縮小
shortcut-global = 全局快速鍵
shortcut-launcher = 臨時視窗
shortcut-add = 添加範本任務
shortcut-edit = 編輯範本任務
shortcut-delete = 刪除範本任務
shortcut-name = 名稱
shortcut-template = 指令範本
shortcut-source = 輸入來源
shortcut-selection = 選中文字
shortcut-clipboard = 剪貼簿
shortcut-fallback = 優先選中文字，否則剪貼簿
shortcut-model = 模型
shortcut-thinking = 思考等級
shortcut-default-model = Pi 預設模型
shortcut-default-thinking = Pi 預設思考等級
shortcut-personal = 個人
shortcut-template-required = 請選擇指令範本。
shortcut-reload-options = 重新讀取選項
temporary-clean-released = 已釋放的工作目錄
temporary-clean = 清理
temporary-clean-help = 將已釋放的臨時工作目錄移到廢紙簍，保留仍在使用的對話。
temporary-cleaned = 已移到廢紙簍的工作目錄
shortcut-tasks = 範本快捷任務

shortcut-select-template = 選擇指令範本

temporary-search-placeholder = 搜尋臨時對話
temporary-search-empty = 沒有匹配的臨時對話
temporary-toggle-input = 切換輸入
temporary-hide = 隱藏
session-search-open = 打開對話

temporary-actions = 操作
temporary-paste-answer = 貼上最後回答
temporary-reveal-workspace = 定位工作目錄
temporary-search-actions = 搜尋操作…
temporary-switch-session = 切換臨時對話
temporary-paste-failed = 回答已複製，自動貼上未完成。請檢查輔助功能權限和目標應用程式，或手動貼上。
temporary-session-1 = 切換到第 1 個臨時對話
temporary-session-2 = 切換到第 2 個臨時對話
temporary-session-3 = 切換到第 3 個臨時對話
temporary-session-4 = 切換到第 4 個臨時對話
temporary-session-5 = 切換到第 5 個臨時對話
temporary-session-6 = 切換到第 6 個臨時對話
temporary-session-7 = 切換到第 7 個臨時對話
temporary-session-8 = 切換到第 8 個臨時對話
temporary-session-9 = 切換到最後一個臨時對話
temporary-send = 發送
temporary-trash = 移到廢紙簍（臨時對話）
settings-key-stop-or-hide = 停止生成 / 隱藏臨時視窗

conversation-queue-title = 待處理（{ $count }）
conversation-queue-steer = 本輪補充
conversation-queue-follow-up = 後續任務
conversation-queue-restore = 全部文字取回草稿
conversation-queue-clear = 清空全部排隊消息
conversation-queue-no-text = 無文字消息
conversation-queue-unavailable = 暫未獲取隊列內容
conversation-queue-text-only = 取回僅恢復文字，排隊圖片無法恢復。

conversation-retry-countdown = 第 { $attempt }/{ $total } 次重試，約 { $seconds } 秒後重試
conversation-retry-waiting = 第 { $attempt }/{ $total } 次重試，等待 Pi
conversation-summary-retry-countdown = 摘要第 { $attempt }/{ $total } 次重試，約 { $seconds } 秒後重試
conversation-summary-retry-waiting = 摘要第 { $attempt }/{ $total } 次重試，等待 Pi

message-details-open = 查看詳情…
message-details-copy-all = 複製全部
tool-detail-command = 指令
tool-detail-content = 內容
tool-detail-changes = 修改

settings-notifications = 通知
settings-notification-help = 系統通知只顯示通用描述，點擊返回來源對話。在前台打開對話後清除未讀標記。
settings-notification-waiting = 後台需要輸入時發送系統通知
settings-notification-failures = 後台任務失敗時發送系統通知
settings-notification-plugins = 將外掛提醒作為後台系統通知發送
settings-notification-attention = 後台需要輸入時請求系統注意力
settings-notification-completion = 回答完成通知
notification-off = 關閉
notification-background = 僅後台
notification-always = 始終
notification-waiting = 等待你的輸入
notification-completed = 回答已完成
notification-failed = 任務失敗，需要查看
notification-plugin = 外掛提醒
notification-unread = 未讀對話
notification-running = 執行中
notification-clear = 清除提醒
notification-session-notices = 對話提醒

settings-icon-theme = 應用程式圖標
settings-icon-theme-help = 更改應用程式內標記和執行時的 macOS Dock 圖標；Finder 與 Windows 任務欄保留打包圖標。
icon-theme-classic = 經典
icon-theme-classic-gradient = 經典漸變
icon-theme-color = Pi 彩色
icon-theme-color-gradient = Pi 漸變
icon-theme-pride = 彩虹
icon-theme-ukraine = 烏克蘭
icon-theme-ukraine-gradient = 烏克蘭漸變

language-traditional-chinese = 繁體中文
language-japanese = 日語
language-korean = 韓語
language-german = 德語
language-french = 法語
language-spanish = 西班牙語
language-portuguese-brazil = 葡萄牙語（巴西）
menu-about = 關於 Gupi
menu-conversation = 對話
menu-edit = 編輯
menu-view = 顯示
menu-window = 視窗
menu-help = 幫助
menu-services = 服務
menu-hide = 隱藏 Gupi
menu-hide-others = 隱藏其他
menu-show-all = 顯示全部
menu-undo = 撤銷
menu-redo = 重做
menu-cut = 剪下
menu-copy = 複製
menu-paste = 貼上
menu-select-all = 全選
menu-minimize = 最小化
menu-zoom = 縮放
menu-fullscreen = 切換全屏
menu-docs = Gupi 使用指南
menu-pi-docs = Pi 文檔
menu-report-issue = 報告問題
menu-logs = 顯示日誌
menu-copy-diagnostics = 複製診斷資訊
settings-permissions = 系統權限
settings-accessibility-help = 讀取選中文字和向其他應用程式貼上可能需要輔助功能權限。在系統設定中允許 Gupi 後，重新執行操作即可。貼上失敗時仍可複製回答。
settings-accessibility-open = 輔助功能設定
settings-notification-permission-help = 若沒有通知或 Dock 角標，請在系統設定中允許 Gupi 的通知和角標。權限會在首次使用相關功能時請求。
settings-notification-permission-open = 系統通知設定
settings-native-language-help = 應用程式語言即時切換。macOS 系統對話框需重啓 Gupi 後生效；Windows 系統對話框跟隨 Windows 顯示語言。
settings-diagnostics-help = 包含應用程式版本、平台、Pi 指令及設定和日誌位置，不包含對話正文、憑據或環境變量。

setup-preferences-title = 語言與外觀
setup-preferences-step = 語言與外觀
setup-desktop-title = 快速鍵與通知
setup-desktop-step = 快速鍵與通知
setup-desktop-help = 稍後可以在設定中更改這些選項。
setup-page-skip = 跳過此頁
setup-skip-all = 使用預設設定
setup-pi-later = 稍後設定 Pi
action-open = 打開

setup-appearance-options = 選擇主題與圖標
setup-pi-install-guide = 安裝 Pi
setup-pi-model-guide = 設定模型訪問

conversation-plugin-message = 外掛訊息
conversation-session-info-command = 對話資訊…
conversation-session-info-title = 對話資訊
conversation-session-info-identity = 身分與位置
conversation-session-info-name = 對話
conversation-session-info-id = Pi 對話 ID
conversation-session-info-directory = 工作目錄
conversation-session-info-file = 對話檔案
conversation-session-info-not-saved = 尚未儲存
conversation-session-info-runtime = 執行設定
conversation-session-info-model = 模型
conversation-session-info-thinking = 思考等級
conversation-session-info-usage = 用量
conversation-session-info-input = 輸入 Token
conversation-session-info-output = 輸出 Token
conversation-session-info-cache-read = 快取讀取
conversation-session-info-cache-write = 快取寫入
conversation-session-info-cost = Pi 回報的費用
conversation-session-info-context = 目前上下文（Token / 視窗 / 使用率）
conversation-session-info-context-value = 已用 { $tokens } / 視窗 { $window } Token（{ $percent }）
conversation-session-info-counts = 對話統計
conversation-session-info-user-messages = 使用者訊息
conversation-session-info-assistant-messages = 助理訊息
conversation-session-info-tool-calls = 工具呼叫
conversation-session-info-tool-results = 工具結果
conversation-session-info-total-messages = 訊息總數

conversation-find = 尋找對話內文
conversation-find-placeholder = 尋找對話內文
conversation-find-count = { $current } / { $total }
conversation-find-previous = 上一處（Shift+Enter）
conversation-find-next = 下一處（Enter）
conversation-find-close = 關閉尋找（Esc）

# Composer resources
command-select-resource = 填入訊息
command-group-files = 檔案與資料夾
resource-files-loading = 正在搜尋專案檔案與資料夾…
resource-open-file = 開啟檔案
resource-skill-content = Skill 指令
resource-template-unreadable = 無法讀取提示詞範本。請重新載入資源後再試。
resource-template-unused-files = 此範本未使用全部檔案引用參數。請補上對應參數（例如 $ARGUMENTS），或移除這些引用後再傳送。
resource-search-files = 搜尋專案檔案與資料夾…
resource-choose-files = 選擇檔案或資料夾…

resource-edit-in-composer = 在這裡選擇資源，參數回訊息輸入框編輯。
resource-template-unclosed-quote = 範本參數中有未閉合的引號，請補全後再傳送檔案引用。

resource-answer-required = 請選擇一個答案後提交。

menu-check-updates = 檢查更新…
updates-title = 更新
updates-check = 檢查更新
updates-automatic = 自動檢查更新
updates-automatic-help = 每小時透過 GitHub 檢查新版本，由你選擇何時下載。
updates-idle = 檢查是否有新版 Gupi。
updates-checking = 正在檢查更新
updates-current = 已是最新版本
updates-unpublished = 尚未發佈正式版本。
updates-available = Gupi { $version } 已發佈
updates-release = 更新說明與下載
updates-install-help = 請使用原來的安裝程式或套件管理器更新 Gupi。
updates-error-network = 無法檢查更新，請檢查網路連線後重試。
updates-error-rate-limit = GitHub 請求次數已達上限，請稍後重試。
updates-error-response = 無法讀取版本資訊，請稍後重試。

updates-install = 下載並安裝…
updates-native-progress = 請在更新視窗中完成更新。
updates-install-failed = 無法完成更新。請重試，或重新開啟 Gupi 並查看日誌。
updates-restart-help = 安裝時會儲存草稿、關閉 Pi 連線並重新啟動 Gupi。
updates-show-progress = 查看更新進度…

shortcut-screenshot = 截圖
screenshot-failed = 無法擷取螢幕，請重試。
screenshot-permission-required = 截圖需要螢幕錄製權限。請在「系統設定 → 隱私權與安全性 → 螢幕錄製」中允許 Gupi，然後重試。如系統提示重新啟動 Gupi，請先重新啟動。

# Project files and source preview
files-title = 檔案
files-panel = 右側面板
files-close = 關閉面板
files-refresh = 重新整理檔案
files-preview = 預覽
files-readonly = 唯讀
files-wrap = 自動換行
files-reload = 重新讀取
files-loading = 正在讀取…
files-expanded = 已展開
files-collapsed = 已收合
files-incomplete = 部分項目無法讀取，點擊重試
files-read-failed = 無法讀取檔案或目錄，請重試
files-unsupported = 此檔案暫不支援文字預覽
files-too-large = 檔案超過預覽上限（1 MiB 或 20,000 行）
files-long-line = 存在超長行，已關閉語法醒目提示
files-no-session = 選擇對話以瀏覽專案檔案

files-actions = 檔案操作
files-copy-path = 複製路徑
files-reveal = 在檔案中定位
files-close-source = 關閉檔案
files-focus-source = 聚焦原始碼
files-conversation = 對話
files-conversation-busy = 對話，正在回覆
files-view = 檢視
files-show-conversation = 顯示對話
files-source = 原始碼

files-link-label = { $name } — 符號連結
files-link-directory-label = { $name } — 資料夾連結，{ $state }
files-link-unavailable-label = { $name } — 連結目標無法使用
files-link-unavailable = 連結目標無法使用

# Settings and shortcuts redesign
settings-page-packages = 套件
settings-app-language = 介面語言
settings-config-location = 位置
settings-temporary-storage = 臨時對話儲存
settings-accessibility = 輔助使用
settings-open-system-settings = 打開系統設定
settings-color-mode-label = 模式
settings-light-help-active = 淺色模式下使用此主題。目前正在使用。
settings-dark-help-active = 深色模式下使用此主題。目前正在使用。
settings-icon-theme-help-other = 更改應用程式內標記；工作列與檔案管理員保留封裝圖示。
settings-notification-background-heading = 背景活動
settings-notification-completion-heading = 回答完成
settings-notification-permission-help-windows = 如果收不到提醒，請在 Windows 設定中允許 Gupi 通知。首次使用該功能時會要求權限。
settings-pi-status-ready = 可用 · { $version } · { $path }
settings-pi-draft = 未儲存
settings-pi-saved-tag = 已儲存
settings-diagnostics = 診斷
settings-open-pi = 打開 Pi 設定
settings-resource-actions = 更多操作
settings-resource-delete-ellipsis = 移到廢紙簍…
settings-resource-from-package = 來自 { $package }
settings-resource-open-package = 顯示所屬套件
settings-key-customized = 已自訂
settings-key-customized-only = 僅已自訂
settings-key-group-system = 系統層級
settings-key-system-help = Gupi 執行時，這些快速鍵在其他應用程式中也可使用。
settings-key-group-workspace = 工作區與檔案
settings-key-reset-all-ellipsis = 全部恢復預設…
settings-key-confirm = 確定
settings-key-unsaved = 未儲存
settings-key-scope-any-window = 任一 Gupi 視窗
settings-key-scope-palette = 主視窗、臨時視窗與啟動視窗
settings-key-scope-conversation = 主視窗與臨時視窗中的對話
settings-key-scope-messages = 訊息列表取得焦點時
settings-key-scope-stop = 主視窗：停止生成 · 臨時視窗：隱藏
settings-key-scope-temporary = 臨時視窗
settings-key-scope-temporary-paste = 臨時視窗；單獨 Enter 在輸入框內不觸發
settings-key-search = 搜尋快速鍵
settings-key-search-keystroke = 依按鍵搜尋
settings-key-search-clear = 清除按鍵搜尋
settings-key-sessions = 切換到臨時對話 1–9
settings-key-sessions-expand = 展開
settings-key-sessions-collapse = 收合
settings-key-show-conflict = 顯示
settings-key-used-by = 已被「{ $command }」使用（{ $scope }）
settings-key-task-actions = 任務操作
settings-key-task-description = /{ $template } · { $source }
settings-key-task-enabled = 啟用任務
shortcut-delete-ellipsis = 刪除範本任務…

# Package catalog
packages-installed-tab = 已安裝
packages-browse-tab = 瀏覽
packages-search = 搜尋套件
packages-source-npm = 結果來自 npm 上帶有 pi-package 關鍵字的套件。
packages-directory = Pi 套件目錄
packages-loading = 正在載入…
packages-retry = 重試
packages-installed = 已安裝
packages-previous = 上一頁
packages-next = 下一頁
packages-back = 返回
packages-latest = 最新發布 { $version }
packages-publisher = 發布者 { $publisher }
packages-repository = 儲存庫
packages-npm = npm 頁面
packages-install-source = 安裝來源
packages-declared = 宣告的內容
packages-undeclared = 套件未宣告內容，安裝後列出。
packages-installed-version = 已安裝 { $version }
packages-files-missing = 檔案遺失
packages-trust = 套件會以你的權限執行程式碼。
packages-done = 已完成。重新載入現有對話或新建對話後生效。
packages-failed = Pi 回報了錯誤：
packages-install = 安裝
packages-copy-source = 複製安裝來源
packages-open-repository = 打開儲存庫
packages-open-npm = 打開 npm 頁面
packages-problem-network = 無法連線到 npm 登錄檔，請檢查網路後重試。
packages-problem-rate-limited = npm 登錄檔限制了請求，請稍後重試。
packages-problem-not-found = 找不到這個套件。
packages-problem-invalid = npm 登錄檔傳回了無法識別的內容。
settings-package-install-source = 從來源安裝…
settings-package-remove-ellipsis = 移除…

# Package browser layout
packages-source-short = npm · pi-package
packages-page = 第 { $page } 頁
packages-page-empty = 本頁無符合條件的套件。
packages-close = 關閉詳細資料
packages-local-unavailable = 本機套件資訊無法使用，請重新整理資源後再操作。
packages-refresh-details = 重新整理詳細資訊

## Pi settings
settings-domain-label = 設定區域
settings-domain-gupi = Gupi
settings-domain-pi = Pi
settings-page-connection = 連線
settings-page-pi-conversation = 對話
settings-page-pi-network = 網路
pi-group-model = 預設模型
pi-group-context = 上下文
pi-group-queue = 訊息佇列
pi-group-images = 圖片
pi-group-proxy = 代理伺服器
pi-group-retry = 重試
pi-field-model = 預設模型
pi-field-model-help = 新對話使用；現有對話保留各自的模型。
pi-field-model-auto = 自動
pi-field-model-unavailable = 此模型目前無法使用。
pi-field-thinking = 預設思考等級
pi-thinking-unsupported = { $level }（所選模型不支援）
pi-field-compaction = 自動壓縮
pi-field-reserve = 壓縮預留 token
pi-field-reserve-help = 壓縮時為模型回覆保留的 token。
pi-field-keep = 保留近期 token
pi-field-keep-help = 壓縮時不會被摘要的近期上下文。
pi-field-steering = 插話訊息
pi-field-follow-up = 排隊訊息
pi-delivery-one = 逐則
pi-delivery-all = 一次全部
pi-field-auto-resize = 自動縮小圖片
pi-field-auto-resize-help = 傳送前將圖片縮放到 2000×2000 以內。
pi-field-block-images = 不向模型傳送圖片
pi-field-proxy = HTTP 代理伺服器
pi-field-proxy-help = 只影響 Pi 發出的請求，不影響 Gupi。
pi-edit-in-global = 在全域中編輯
pi-value-unset = 未設定
pi-field-retry = 自動重試
pi-field-max-retries = 最大重試次數
pi-field-base-delay = 初始延遲（毫秒）
pi-field-max-delay = 最大延遲（毫秒）
pi-source-default = 預設
pi-source-global = 全域
pi-source-project = 專案
pi-unsaved = 未儲存
pi-changed-on-disk = 讀取後檔案中的值已被修改。
pi-invalid-count = 請輸入 0 或更大的整數。
pi-menu-revert = 復原變更
pi-menu-pin-project = 設為專案值
pi-menu-pin-global = 設為全域值
pi-menu-inherit = 使用全域值
pi-menu-default = 使用預設值
pi-field-actions = 欄位操作
pi-scope-global = 全域
pi-scope-label = 設定範圍
pi-trust-untrusted = 未信任
pi-trust-untrusted-help = 信任專案前，Pi 不會使用這些專案設定。
pi-trust-action = 信任專案…
pi-trust-title = 信任「{ $name }」？
pi-trust-body = Pi 將載入此資料夾的專案設定和資源、安裝專案套件並執行專案擴充功能。上層資料夾不會被信任。
pi-trust-confirm = 信任專案
pi-settings-save = 儲存
pi-settings-discard = 捨棄
pi-settings-file-actions = 更多操作
pi-settings-open-file = 開啟 settings.json
pi-settings-reload = 重新讀取
pi-settings-invalid = 請先修正標示的欄位再儲存。
pi-settings-conflict = 部分設定在讀取後被其他程式修改。請重新讀取後再儲存。
pi-settings-locked = 設定檔正被其他程序使用，請稍後再試。
pi-settings-unsaved-title = 儲存 Pi 設定？
pi-settings-unsaved-body = Pi 設定有未儲存的變更。
pi-settings-loading = 正在讀取…
pi-settings-unsupported = 在此編輯 Pi 設定需要 Pi 1.1.0 或更新版本。目前版本：{ $version }。
pi-settings-unavailable = Pi 無法使用，請檢查「連線」頁。
pi-models-loading = 正在讀取模型…
pi-models-failed = 無法讀取模型：{ $error }
pi-models-retry = 重試
pi-project-settings = 專案 Pi 設定…

pi-scope-choose-folder = 選擇專案資料夾…
pi-scope-choose-folder-action = 選擇
settings-resource-group-project = 本專案
settings-resource-group-inherited = 繼承自全域
settings-source-project = 本專案
settings-resource-same-global = 與全域同名 · 專案優先
settings-resource-same-project = 與專案同名 · 專案優先
settings-resource-not-loaded = 未載入（專案未信任）
settings-resource-edit-global = 在全域中編輯
settings-resource-none-project = 此專案的 .pi 中尚無此類資源。
settings-system-project-file = 專案檔案
settings-system-uses-global = 使用全域
settings-system-use-global = 使用全域
settings-system-use-global-title = 改用全域 { $file }？
settings-system-use-global-body = 專案檔案將移至垃圾桶，Pi 改為回退到全域檔案。
settings-editor-changed = 檔案在開啟後已被修改，未儲存。可以重新讀取（放棄此處修改）或取消。
settings-editor-reload = 重新讀取
settings-editor-unsaved-title = 儲存對 { $file } 的修改？
settings-editor-unsaved-body = 正文修改尚未儲存。

pi-project-settings-folder = 專案 Pi 設定 — { $folder }

packages-install-project = 安裝到專案 { $folder }
packages-project-delta = 全域套件篩選設定 · 唯讀
packages-project-filtered = 在此專案中被篩選
packages-project-replaced = 由專案取代
packages-project-replaces = 取代全域套件

settings-resource-disabled = 已停用
settings-theme-heading = 獨立主題
settings-project-package-empty = 此專案沒有套件。
