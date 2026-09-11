# post-share-plugin — 共有テンプレート(サンプル)

スロット **`post.share`**(CONTRACT.md §2.8)。投稿の **共有 FOW** の「プラグイン」節に(マニフェストの
宣言だけで)並び、選ぶと共有 FOW のインラインに FOW を出して、文面を **テンプレート** で整えて共有先へ渡す。

- テンプレート: 「リンクのみ」(`<link>`)/「引用(本文つき)」(`「<本文 80 文字まで>」\n<link>`)/
  「作者に言及」(`@<handle>@<instance> さんの投稿\n<link>`)。選んだものは `storage.safe` の `template` に覚えて次回も使う
- 共有先: **コンポーザーへ**(`composer.open {token, text}`)/ **コピー**(`clipboard.set` + `toast`)/
  **ブラウザで開く**(`url.open {url: link}`)/ **AsTell へ**(`astell.share {kind:"text", payload:{text}}`)
- 上部に投稿のプレビュー(作者 + 本文 60 文字)、下部に今の文面(`Text`。テンプレートを変えると `setProp` で差し替わる)

## call / ホスト関数の流れ

```
ホスト → plugin.manifest                       ← {id:"sample.post-share", slots:["post.share"], permissions:["fow.widget","storage.safe"]}
ホスト → post.share {token, post, account, link}
            プラグイン → storage.get template
            プラグイン → fow.open {id:"share-template", spec:{title:"共有テンプレート", initialHeightFactor:0.55}, onClosed:"panelClosed", root}
                                                   ← {}
FOW の ListTile → tpl.link / tpl.quote / tpl.mention
            → {patch:true}(surface patch: 各行の title のチェック + result の text)→ storage.set template
FOW ボタン → share.composer                    → composer.open {token, text}               ← {}
FOW ボタン → share.copy                        → clipboard.set {text} → toast               ← {}
FOW ボタン → share.browser                     → url.open {url: link}                        ← {}
FOW ボタン → share.astell                      → astell.share {kind:"text", payload:{text}} → toast ← {}
ホスト → post.share.close {token}              → fow.close(開いていれば)+ 状態を捨てる    ← {}
ホスト → panelClosed / lifecycle               ← {}(状態を捨てる)
```

| ノード key | 種類 | fn |
|---|---|---|
| `preview-author` / `preview-body` / `account` | `Text` | — |
| `tpl-link` / `tpl-quote` / `tpl-mention` | `ListTile`(選択中は title の先頭に `✓`) | `tpl.link` / `tpl.quote` / `tpl.mention` |
| `result` | `Text`(今の文面) | — |
| `btn-composer` / `btn-copy` / `btn-browser` / `btn-astell` | `Button` | `share.composer` / `share.copy` / `share.browser` / `share.astell` |

FOW が閉じている(`post.share.close` / `panelClosed` の後)ときのボタン / テンプレートのタップは何もしない。

## ビルド / テスト

```sh
cargo test -p post-share-plugin
cargo build --release --target wasm32-unknown-unknown -p post-share-plugin
#  → target/wasm32-unknown-unknown/release/post_share_plugin.wasm
```

native テストは `rasq_plugin_kit::set_host_hook` でホスト関数を差し替え、テンプレート文面(80 文字での切り詰め、
ハンドルの `@user@host` 化)、`post.share` → `fow.open` の root、`tpl.quote` → surface patch + `storage.set`、
`share.composer` → `composer.open {token, text}`、`post.share.close` → `fow.close` を検証している。

## 端末で試す(rasq-client)

1. サイドバー → **WASMプラグイン試験 (dev)** → `.wasm のパスからインポート` に `post_share_plugin.wasm` の絶対パス → **読み込み**
2. **スロットに登録**(権限 `fow.widget` / `storage.safe` を許可)
3. 投稿のアクションバー右端(共有)→ 共有 FOW の **プラグイン** 節 → 「共有テンプレート(サンプル)」
4. テンプレートを選び、「コンポーザーへ」で選んだアカウントのコンポーザーに文面が入る(コピー / ブラウザ / AsTell も)

---

## このリポジトリの形

- クレートはルート(`src/`)、共通部品 [`rasq-plugin-kit`](kit) は `kit/` に同梱(開発用モノレポから写している)。
- 契約は [`CONTRACT.md`](CONTRACT.md)(スロットプラグイン契約 v0.1)。
- ビルド済み `.wasm` は [`dist/`](dist)。`cargo build --release --target wasm32-unknown-unknown` で更新。
