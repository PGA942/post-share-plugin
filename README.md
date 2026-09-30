# post-share-plugin — 共有テンプレート(サンプル)

スロット **`post.share`**([`docs/slots.md`](https://gitlab.com/orgate-question/rasq-sdk-sample/-/blob/main/docs/slots.md) §2.8)。
投稿の **共有 FOW** の「プラグイン」節に(マニフェストの宣言だけで)並び、選ぶと共有 FOW のインラインに FOW を出して、
文面を **テンプレート** で整えて共有先へ渡す。

- テンプレート: 「リンクのみ」(`<link>`)/「引用(本文つき)」(`「<本文 80 文字まで>」\n<link>`)/
  「作者に言及」(`@<handle>@<instance> さんの投稿\n<link>`)。選んだものは `storage.safe` の `template` に覚えて次回も使う
- 共有先: **コンポーザーへ**(`composer.open {token, text}`)/ **コピー**(`clipboard.set` + `toast`)/
  **ブラウザで開く**(`url.open {url: link}`)/ **AsTell へ**(`astell.share {kind:"text", payload:{text}}`)
- 上部に投稿のプレビュー(作者 + 本文 60 文字)、下部に今の文面(`Text`。テンプレートを変えると `setProp` で差し替わる)

## このリポジトリの形(`rasq` ワークスペース)

```
rasq.toml                         schema = "rasq.workspace/1" / [author] namespace = "sample"
                                  [[plugins]] name = "post-share-plugin" / slot = "post.share" / id = "sample.post-share"
Cargo.toml                        rasq CLI が rasq.toml から作り直す([workspace] / [profile.release])
kit/                              rasq-sdk-sample の rust/rasq-plugin-kit の写し
plugins/post-share-plugin/
  Cargo.toml                      crate-type = ["cdylib", "rlib"] / rasq-plugin-kit = { path = "../../kit" }
  manifest.json                   plugin.manifest の答えそのもの
  src/lib.rs                      include_str!("../manifest.json") + handle の match + #[cfg(test)]
dist/                             rasq build --raw --no-sign の出力(.rasqpkg と生の .wasm)
CONTRACT.md                       SDK の docs/slots.md へのポインタ
```

マニフェストは `plugins/post-share-plugin/manifest.json` の 1 か所。`src/lib.rs` は `include_str!` でそれを
そのまま返すので、`rasq check` の往復検査(`plugin.manifest` ⇄ `manifest.json`)が常に一致する。

## 契約(`docs/slots.md` §2.8)

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
`token` の無い `post.share` は `D01`、知らない fn は `C01`。

## ビルドと確認

```sh
rasq check                 # wasm32 ビルド → export → plugin.manifest 往復 → スキーマ / 登録簿 → cargo test
rasq test                  # cargo test だけ
rasq build --raw --no-sign # dist/sample.post-share-0.1.0.rasqpkg + dist/post_share_plugin.wasm
rasq inspect dist/sample.post-share-0.1.0.rasqpkg
```

native テストは `rasq_plugin_kit::set_host_hook` でホスト関数を差し替え、テンプレート文面(80 文字での切り詰め、
ハンドルの `@user@host` 化)、`post.share` → `fow.open` の root、`tpl.quote` → surface patch + `storage.set`、
`share.composer` → `composer.open {token, text}`、`post.share.close` → `fow.close`、
`plugin.manifest` ⇄ `manifest.json` の一致を検証している。

実 wasm を直に叩く(`rasq call`。ホスト関数の答えは `--reply`、続けて呼ぶのは `--then`):

```sh
rasq call post-share-plugin plugin.manifest
#  → manifest.json と同じ JSON

rasq call post-share-plugin post.share --args @args.json \
  --reply 'storage.get={"value":null}' --reply 'storage.set={}' --reply 'fow.open={"opened":true}' \
  --then tpl.quote --then share.composer --then share.copy --then post.share.close
#  post.share        → {} / host storage.get + fow.open(root に tpl-link ✓ / result = link)
#  tpl.quote         → {"patch":true} / host storage.set {"value":"quote"}
#                      patch: tpl-* の title と result の text(「<本文>」\n<link>)
#  share.composer    → {} / host composer.open {"token":"tk-1","text":"「…」\n<link>"}
#  share.copy        → {} / host clipboard.set + toast
#  post.share.close  → {} / host fow.close {"id":"share-template"}

rasq call post-share-plugin post.share --args '{}'   # → ERROR D01 token required
rasq call post-share-plugin tpl.nope                 # → ERROR C01 unsupported fn
```

`args.json` の例:

```json
{ "token": "tk-1",
  "post": {"url": "https://example.social/@alice/123", "sns": "mastodon", "instance": "example.social",
           "postId": "123", "author": "アリス", "handle": "alice", "body": "きょうは良い天気です", "media": []},
  "account": {"sns": "mastodon", "instance": "mine.example", "handle": "me"},
  "link": "https://example.social/@alice/123" }
```

## 端末での試し方(rasq-client)

1. `rasq dev` が出す絶対パス(`dist/post_share_plugin.wasm`)を控える
2. サイドバー → **WASMプラグイン試験 (dev)** → `.wasm のパスからインポート` にそのパス → **読み込み**
3. **スロットに登録**(権限 `fow.widget` / `storage.safe` を許可)
4. 投稿のアクションバー右端(共有)→ 共有 FOW の **プラグイン** 節 → 「共有テンプレート(サンプル)」
5. テンプレートを選び、「コンポーザーへ」で選んだアカウントのコンポーザーに文面が入る(コピー / ブラウザ / AsTell も)

## 掲載(審査)に出す

`rasq build` が出す `dist/sample.post-share-0.1.0.rasqpkg`(zstd tar + `bundle.json` `rasq.bundle/1` + `SHA512SUMS`)が
そのまま掲載できる形。`rasq inspect <pkg>` で目次と中身を確かめ(末尾が `目次と中身: ok`)、
`rasq publish --dry-run` で要求を `dist/publish/` に書き出せる。
開発者署名(`*.sig.json`)は手元でだけ作るものなのでコミットしない(`.gitignore`)。
