# easy-helix

> 這份文件只寫 easy-helix 新增和修改的部分。Helix 原本的功能、安裝方式和說明，請看 **[README.en.md](./README.en.md)**。

easy-helix 是 [Helix](https://github.com/helix-editor/helix) 的個人 fork，放在 [scott1028/helix](https://github.com/scott1028/helix) 的 `easy-helix` branch。它在 Helix 上加了 VSCode / Sublime 風格的畫面和鍵位。

你可以把它想成在原廠車上加裝配備：引擎沒動，Helix「先選取、再動作」的編輯模型不變；多出來的是側邊欄、tab、滑鼠、Markdown 預覽和一套出廠預設。

## 一張表看完改了什麼

| 項目 | 你會看到什麼 | 完整鍵位與設定 |
|---|---|---|
| 檔案樹 sidebar | 左邊一欄目錄樹，`Space-t` 開關 | [keymap: Sidebar](./book/src/keymap.md#sidebar)、[`[editor.sidebar]`](./book/src/editor.md#editorsidebar-section) |
| tab 列 | 開兩個以上的檔案時，上方出現一排 tab，可以用鍵盤或滑鼠切換 | [keymap: Tabs](./book/src/keymap.md#tabs) |
| 滑鼠 | 點 tab、點檔案樹、拖 sidebar 邊框、用滾輪捲檔案樹 | 本文「滑鼠能做什麼」 |
| 開 binary 檔 | 控制字元畫成 `␀`、`␛` 這類符號，畫面不再亂掉 | 本文「開 binary 檔不再弄壞畫面」 |
| `Space-v` Markdown 預覽 | 用 Typora（或你設定的程式）打開目前的 `.md` | [`[editor]` 的 `markdown-preview`](./book/src/editor.md#editor-section) |
| 出廠預設鍵 | `Ctrl-w` 關 tab、`yy`、`dd`、`F5` 重讀檔案、`Alt-Left/Right` 切 tab 等 | [keymap: easy-helix default keys](./book/src/keymap.md#easy-helix-default-keys) |
| Release | 推 tag 就自動出 Linux AppImage 和 Apple silicon 的壓縮檔 | 本文「發新版」 |

## 下載和安裝

每個 release 只出兩種檔案，都已經內含編好的 grammar（tree-sitter 的語法解析器），下載就能用，不用另外跑 `hx --grammar fetch`。
請到 [Releases](https://github.com/scott1028/helix/releases) 下載。

| 檔案 | 平台 | 怎麼用 |
|---|---|---|
| `easy-helix-linux-x86_64.AppImage` | Linux x86_64 | `chmod +x` 之後當成 `hx` 執行；系統沒有 FUSE 時，先設 `APPIMAGE_EXTRACT_AND_RUN=1` |
| `easy-helix-macos-aarch64.tar.xz` | macOS（Apple silicon） | 解壓縮後，`hx` 要跟 `runtime` 資料夾放在一起 |
| `SHA256SUMS` | — | 用 `sha256sum -c SHA256SUMS` 檢查下載的檔案 |

幾個容易踩的坑：
- **macOS 擋住不給執行**：binary 沒有簽章，跑 `xattr -d com.apple.quarantine hx` 解除。
- **設定檔位置**：跟 Helix 一樣是 `~/.config/helix/config.toml`。你在裡面寫的設定會蓋過 easy-helix 的出廠預設。

## 版號怎麼看

tag 的格式是 `easy-helix-v<version>.<N>`，例如 `easy-helix-v25.07.1.1`。

- 前三段是 easy-helix 所根據的 upstream Helix 版本，也就是 `Cargo.toml` 的 `version`（`25.7.1`）寫成 Helix 的日曆格式，`hx --version` 會印出 `25.07.1`。
- 最後的 `N` 是 fork 自己的改版次數，從 1 開始。`easy-helix-v25.07.1.1` 的下一版是 `easy-helix-v25.07.1.2`；換到新的 upstream 版本時，`N` 從 1 重新算。

## 畫面配置

這張圖是同時打開 sidebar 和 tab 列時的版面，只表示結構，不是實際的像素位置。

```text
+------------------------------------------------------------------+
| a.md  b.md  c.md                                   <- bufferline |
+---------------------+--------------------------------------------+
| helix               |  1  # Title                                |
|  v book             |  2                                         |
|    > src            |  3  Hello                                  |
|  > helix-term       |                                            |
|    README.md        |                                            |
|            sidebar <|> drag this border                          |
+---------------------+--------------------------------------------+
| NOR  README.md                                              1:1  |
| book/src/keymap.md                          <- selected path row |
+------------------------------------------------------------------+
```

- bufferline 就是 tab 列：Helix 把開著的檔案（buffer）排成一排。
- 檔案樹的第一列是根目錄名稱。實際畫面用 `▾`（已展開）和 `▸`（未展開）標目錄。
- 最下面那列只在 sidebar 有 focus（鍵盤輸入正送到它）時出現，顯示選到的檔案相對於根目錄的路徑。

## Sidebar：在左邊看整個專案

sidebar 是以目前工作目錄為根的檔案樹，讓你不開 picker 也能看到專案結構、直接點開檔案。
要顯示哪些檔案，跟著 `[editor.file-explorer]` 的設定走（例如 hidden、gitignore）。

```toml
[editor.sidebar]
visible = false   # 啟動時要不要顯示
width = 30        # 欄寬，最多半個畫面；小於 10 欄就不畫
```

| 鍵 | 作用 |
|---|---|
| `Space-t` | 顯示或隱藏 sidebar |
| `Ctrl-e` | 在 sidebar 和編輯區之間切換 focus |
| `Ctrl-Left`、`Space-w h` | 從最左邊的 view 進到 sidebar |
| `Ctrl-Right`、`Space-w l` | 從 sidebar 回到編輯區 |

sidebar 有 focus 時：
- **移動**：`j`/`k`/`Up`/`Down`（可以加次數）；`Home`/`G`/`End` 到頭尾；`Ctrl-u`/`Ctrl-d` 移半頁；`PageUp`/`PageDown` 移一頁。
- **展開與開檔**：`h`/`Left` 收合目錄或回上一層；`l`/`Right` 展開目錄或開檔；`Enter`/`o` 展開、收合或開檔。
- **其他**：`R` 從硬碟重讀；`Escape` 回到編輯區。

你會注意到的行為：
- **自動加寬**：有 focus 時，sidebar 會加寬到放得下檔名（最多半個畫面），離開 focus 就縮回去。
- **長檔名**：放不下時中間用 `…` 省略，頭尾都看得到。
- **只放行不改內容的指令**：有 focus 時，只有 `:` 指令、picker、`Space-t`、切 tab、切視窗這類指令會執行，免得你以為在操作檔案樹，卻改到編輯區的檔案。開檔或切 tab 之後，focus 會回到編輯區。

## Tab 列：在開著的檔案之間切換

tab 列讓你看得到開了哪些檔案，並且一鍵切過去。
upstream 的 `bufferline` 預設是 `never`（不顯示）；easy-helix 改成 `multiple`，開兩個以上的檔案就會出現。

| 鍵 | 作用 |
|---|---|
| `Alt-Left`、`Alt-Right` | 上一個 / 下一個 tab |
| `Ctrl-PageUp`、`Ctrl-PageDown` | 上一個 / 下一個 tab |
| `Ctrl-Shift-Tab`、`Ctrl-Tab` | 上一個 / 下一個 tab（terminal 要支援 kitty keyboard protocol） |
| `Alt-1` .. `Alt-9` | 跳到第 N 個 tab，沒那麼多就跳到最後一個（`:buffer-goto N`） |
| `Ctrl-w` | 關掉目前的 tab，有未存檔的修改會擋下來（`:buffer-close`） |
| `Alt-w` | 強制關掉，未存檔的修改直接丟掉（`:buffer-close!`） |

- `:buffer-goto` 是 easy-helix 新增的 typable command（在 `:` 後面打的指令）。
- **terminal 會搶鍵**：有些 terminal 把切分頁的鍵留給自己。例如 Ghostty 預設攔下 `Ctrl-PageUp/PageDown`、`Ctrl-Tab`、`Ctrl-Shift-Tab`、`Alt-1..9`，這些鍵送不到 hx，請改用 `Alt-Left/Right` 或 Helix 原本的 `gn`/`gp`。
- tab 太多時 tab 列會捲動，確保目前的 tab 看得到；切換時盡量不移動，免得整排一直跳。

## 滑鼠能做什麼

滑鼠讓 sidebar 和 tab 列可以直接點，不用記鍵。前提是 `mouse = true`，這也是 Helix 的預設值。

| 在哪裡 | 動作 | 結果 |
|---|---|---|
| tab | 左鍵 | 切到那個 tab |
| tab | 中鍵 | 關掉那個 tab，有未存檔的修改會擋下來並提示 |
| sidebar 的列 | 左鍵 | 選取那一列：目錄就展開或收合，檔案就打開 |
| sidebar | 滾輪 | 捲動檔案樹，一次捲 `scroll-lines` 行 |
| sidebar 右邊框 | 拖曳 | 調整寬度，不會搶走 focus；這個寬度記到 Helix 結束 |

從 tab 列或 sidebar 開始的點擊，後續的拖曳和放開事件都會被吃掉，不會誤觸 Helix 原本的拖曳選取、放開時複製、中鍵貼上。

## 開 binary 檔不再弄壞畫面

在 upstream Helix 裡打開圖片這類 binary 檔，整個畫面會亂掉。
原因是檔案裡的控制字元（例如 ESC、BS）被原樣送給 terminal，terminal 照著執行，游標就亂跳。

```text
修正前：檔案裡的 ESC ──原樣送出──► terminal 執行它 ──► 游標亂跳、畫面錯位
修正後：檔案裡的 ESC ──換成 ␛──► terminal 只是印出來 ──► 每一行都在原位
```

- **對照規則**：C0（`U+0000`–`U+001F`）換成 Control Pictures `U+2400` 加上原本的碼（`␀`、`␈`、`␛`…）；DEL 換成 `␡`；C1（`U+0080`–`U+009F`）換成 `�`。
- **只改畫面**：文件內容、游標、選取都不變。從命令列、picker、sidebar 打開都適用。

## `Space-v`：預覽 Markdown

`Space-v` 用外部程式打開目前的 Markdown 檔，預設是 Typora。
Typora 會跟著檔案變動自動重新整理，所以開一次之後，每次 `:w` 預覽就會更新。
下圖的「檢查」依序是：目前的檔案是 Markdown、它有檔案路徑、`markdown-preview` 的程式在 `PATH` 裡。

```text
Space-v
  ├─ 檢查不通過 ──► 狀態列顯示錯誤
  ├─ 沒有未存檔的修改 ──► 直接開預覽
  └─ 有未存檔的修改 ──► 右下角問「Save changes to preview?」
       ├─ y ──► :write，寫完後開預覽
       └─ 其他鍵 ──► 取消，什麼都不寫
```

```toml
[editor]
# 第一個是程式，其餘是參數；參數支援 %{...} 展開，要寫 % 本身就寫 %%
markdown-preview = ["typora", "%{file_path_absolute}"]
```

- **找不到程式時**：`Space` 選單裡的 `v` 會變灰（用主題的 `ui.text.inactive` 顏色），按下去顯示錯誤。
- **不會被連帶關掉**：預覽程式跑在自己的 process group，關掉 Helix 或 terminal 都不影響它。
- **macOS**：Typora 在 Mac 上沒有 `typora` 指令，請設成 `markdown-preview = ["open", "-a", "Typora", "%{file_path_absolute}"]`。

sidebar 有 focus 時 `Space-v` 不會執行，因為它預覽的是目前的 buffer，不是 sidebar 選到的檔案。

## 出廠預設鍵：不用寫 config 就順手

easy-helix 內建一套 VSCode 和 Vim 習慣的鍵，疊在 Helix 原本的鍵上面。你自己的 config 再疊在最上層。

```text
你的 config.toml          <- 最上層，寫了就以它為準
easy-helix 出廠鍵         <- keymap::easy_helix()
Helix 原本的鍵            <- keymap::default()
```

上層蓋過下層，所以你在 config 寫 `y = "yank"` 就能拿回 Helix 原本的行為。
這些鍵只改 Normal mode：select mode 的 `y`、`d` 還是複製、刪除選取，insert mode 的 `Ctrl-w` 還是刪前一個字。

| 鍵 | 作用 | 蓋掉的 Helix 鍵 |
|---|---|---|
| `Ctrl-w` | 關 tab（`:buffer-close`） | Window 選單，改用 `Space-w` |
| `Alt-Left`、`Alt-Right` | 上一個 / 下一個 tab | 選前後的語法節點，`Alt-p`/`Alt-n` 還在 |
| `F5` | 從硬碟重讀（`:reload`） | — |
| `Ctrl-Left/Down/Up/Right` | 切到那一邊的 split 或 sidebar | — |
| `Ctrl-n` | 在右邊開一個新的空白 buffer（`:vnew`） | — |
| `Ctrl-\` | 垂直分割（`vsplit`） | — |
| `yy` | 複製整行 | `y`（複製選取） |
| `dd` | 刪除整行 | `d`（刪除選取） |
| `Ctrl-r` | redo | — |
| `Ctrl-q` | 關 view，丟掉未存檔的修改（`:quit!`） | — |
| `Alt-w` | 關 tab，丟掉未存檔的修改（`:buffer-close!`） | — |
| `Ctrl-x` | 存檔並關 view（`:write-quit!`） | `Ctrl-x`（數字減一） |
| `L` | 顯示或隱藏行號 | — |
| `W` | 切換 soft wrap | `W`（跳到下一個 WORD 開頭） |
| `Alt-l` 再按 `m`/`t`/`j`/`p` | 語言設成 Markdown / TypeScript / JavaScript / Python | — |

想拿回 Helix 原本的鍵，在 `config.toml` 這樣寫：

```toml
[keys.normal]
y = "yank"
d = "delete_selection"
W = "move_next_long_word_start"
C-x = "decrement"
```

## Space 選單多了什麼

| 鍵 | 作用 |
|---|---|
| `Space-t` | 顯示或隱藏 sidebar |
| `Space-v` | 預覽 Markdown |
| `Space-E` | 在目前 buffer 的目錄開 file explorer |

upstream master 把 `Space-E` 移到了 `Space-.`。easy-helix 兩個都保留，維持 25.07 正式版的手感。

## 發新版

推一個 tag，GitHub Actions 就會 build 並建立 release，不用手動上傳檔案。

```text
git push origin easy-helix-v25.07.1.N
  │ 觸發
  ▼
.github/workflows/easy-helix-release.yml
  ├─ 1. 檢查 tag 和 Cargo.toml 的版本一致
  ├─ 2. 抓 grammar 原始碼
  ├─ 3. build Linux x86_64 AppImage（ubuntu-22.04）和 macOS aarch64 tar.xz
  └─ 4. 寫 SHA256SUMS，建立 release
```

```sh
git push origin easy-helix
git tag -a easy-helix-v25.07.1.2 -m easy-helix-v25.07.1.2
git push origin easy-helix-v25.07.1.2
```

也可以不推 tag，直接手動跑 workflow：

```sh
gh workflow run easy-helix-release.yml --ref easy-helix -f tag=easy-helix-v25.07.1.2
```

- **Linux 用 `ubuntu-22.04` build**：glibc 需求比較低，舊一點的發行版也能跑。
- **gh-pages**：`gh-pages.yml` 只在 `helix-editor/helix` 跑，推 easy-helix 的 tag 時會跳過。
