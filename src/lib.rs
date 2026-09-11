//! post-share-plugin — 投稿の **共有 FOW** から呼ばれる文面テンプレート(スロット `post.share`)のサンプル。
//!
//! | 項目 | 入口 |
//! |---|---|
//! | マニフェスト | `plugin.manifest` |
//! | 共有 FOW の行を選んだ | `post.share {token, post, account, link}` → `storage.get template` → `fow.open`(共有 FOW のインライン子) |
//! | テンプレートを選ぶ | `tpl.link` / `tpl.quote` / `tpl.mention` → 文面を `setProp` する surface patch + `storage.set template` |
//! | コンポーザーへ | `share.composer` → `composer.open {token, text}` |
//! | コピー | `share.copy` → `clipboard.set {text}` + `toast` |
//! | ブラウザで開く | `share.browser` → `url.open {url: link}` |
//! | AsTell へ | `share.astell` → `astell.share {kind:"text", payload:{text}}` |
//! | 共有 FOW が閉じた | `post.share.close {token}` → `fow.close` + 状態を捨てる(`panelClosed` も同じ) |
//!
//! ビルド: `cargo build --release --target wasm32-unknown-unknown -p post-share-plugin`
//! 契約: `../CONTRACT.md` §2.8 / §3 / §4。

use rasq_plugin_kit::{err, fow_spec, host, manifest, rasq_plugin, rrl, serde_json::{json, Value}, unsupported, Surface};
use std::sync::Mutex;

const PLUGIN_ID: &str = "sample.post-share";
const VERSION: &str = "0.1.0";

/// プレビューに出す本文の先頭文字数 / 引用に入れる本文の上限。
pub const PREVIEW_CHARS: usize = 60;
pub const QUOTE_CHARS: usize = 80;

/// `storage.safe` に覚える直近のテンプレート。
const STORAGE_KEY: &str = "template";

static PANEL: Surface = Surface::new("share-template");

// ---------------------------------------------------------------------------
// テンプレート
// ---------------------------------------------------------------------------

/// 文面テンプレート。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Template {
    /// リンクだけ。
    #[default]
    Link,
    /// 本文(80 文字まで)を「」で引用してリンク。
    Quote,
    /// 作者のフルハンドルに言及してリンク。
    Mention,
}

impl Template {
    pub const ALL: [Template; 3] = [Template::Link, Template::Quote, Template::Mention];

    /// storage / ノード key / fn 名に使う id。
    pub fn id(self) -> &'static str {
        match self {
            Template::Link => "link",
            Template::Quote => "quote",
            Template::Mention => "mention",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Template::Link => "リンクのみ",
            Template::Quote => "引用(本文つき)",
            Template::Mention => "作者に言及",
        }
    }
    pub fn from_id(id: &str) -> Option<Template> {
        Template::ALL.iter().copied().find(|t| t.id() == id)
    }
    fn node_key(self) -> String {
        format!("tpl-{}", self.id())
    }
    fn fn_name(self) -> String {
        format!("tpl.{}", self.id())
    }
}

/// 文面の材料(`post.share` の args から取り出したもの)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShareInput {
    /// 投稿者の表示名(`post.author`)。
    pub author: String,
    /// 投稿者のハンドル(`post.handle`。`@` 付き / `user@host` 形でも可)。
    pub handle: String,
    /// 投稿のインスタンス(`post.instance`)。
    pub instance: String,
    pub body: String,
    pub link: String,
}

/// 先頭 `max` 文字(Unicode スカラー値)。切ったら末尾に `…`。
pub fn truncate_chars(s: &str, max: usize) -> String {
    let mut it = s.chars();
    let head: String = it.by_ref().take(max).collect();
    if it.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

/// `@handle@instance` の形にそろえる。`handle` に既に `@host` が付いていればそのまま。
pub fn full_handle(handle: &str, instance: &str) -> String {
    let h = handle.trim().trim_start_matches('@');
    if h.contains('@') || instance.is_empty() {
        format!("@{h}")
    } else {
        format!("@{h}@{instance}")
    }
}

/// テンプレートを適用した文面。
pub fn template_text(t: Template, input: &ShareInput) -> String {
    match t {
        Template::Link => input.link.clone(),
        Template::Quote => format!("「{}」\n{}", truncate_chars(input.body.trim(), QUOTE_CHARS), input.link),
        Template::Mention => format!("{} さんの投稿\n{}", full_handle(&input.handle, &input.instance), input.link),
    }
}

// ---------------------------------------------------------------------------
// 状態(開いている共有 1 回ぶん)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct State {
    /// 直近の `post.share` の token(`composer.open` に返す)。
    token: String,
    input: ShareInput,
    /// 選んだアカウント(`account.handle@instance`。表示用)。
    account: String,
    template: Template,
    open: bool,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut guard = match STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    f(guard.get_or_insert_with(State::default))
}

fn str_of(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").to_string()
}

fn input_from_args(args: &Value) -> ShareInput {
    let post = &args["post"];
    ShareInput {
        author: str_of(post, "author"),
        handle: str_of(post, "handle"),
        instance: str_of(post, "instance"),
        body: str_of(post, "body"),
        link: {
            let l = str_of(args, "link");
            if l.is_empty() { str_of(post, "url") } else { l }
        },
    }
}

fn account_label(args: &Value) -> String {
    let a = &args["account"];
    let handle = str_of(a, "handle");
    if handle.is_empty() {
        String::new()
    } else {
        full_handle(&handle, a["instance"].as_str().unwrap_or(""))
    }
}

// ---------------------------------------------------------------------------
// RRL(FOW の中身)
// ---------------------------------------------------------------------------

fn section(key: &str, title: &str) -> Value {
    rrl::styled_text(key, title, 13.0, true, Some("#888888"))
}

/// テンプレート行のタイトル(選択中は先頭にチェック)。
fn tile_title(t: Template, selected: Template) -> String {
    format!("{} {}", if t == selected { "✓" } else { "　" }, t.label())
}

fn template_tile(t: Template, selected: Template) -> Value {
    rrl::list_tile(&t.node_key(), &tile_title(t, selected), None, Some(&t.fn_name()))
}

fn preview_body(input: &ShareInput) -> String {
    truncate_chars(input.body.trim(), PREVIEW_CHARS)
}

fn panel_root(s: &State) -> Value {
    let author = if s.input.author.is_empty() { full_handle(&s.input.handle, &s.input.instance) } else { s.input.author.clone() };
    let mut children = vec![
        rrl::styled_text("preview-author", author, 14.0, true, None),
        rrl::styled_text("preview-body", preview_body(&s.input), 12.0, false, Some("#888888")),
    ];
    if !s.account.is_empty() {
        children.push(rrl::styled_text("account", format!("共有するアカウント: {}", s.account), 11.0, false, Some("#888888")));
    }
    children.push(section("sec-template", "テンプレート"));
    children.extend(Template::ALL.iter().map(|t| template_tile(*t, s.template)));
    children.push(section("sec-result", "文面"));
    children.push(rrl::text("result", template_text(s.template, &s.input)));
    children.push(rrl::row(
        "row-actions",
        vec![
            rrl::button("btn-composer", "コンポーザーへ", "share.composer"),
            rrl::button("btn-copy", "コピー", "share.copy"),
        ],
    ));
    children.push(rrl::row(
        "row-actions-2",
        vec![
            rrl::button("btn-browser", "ブラウザで開く", "share.browser"),
            rrl::button("btn-astell", "AsTell へ", "share.astell"),
        ],
    ));
    rrl::scroll("root-scroll", false, vec![rrl::column("root", children)])
}

/// 本体ツリーは使わない(UI は共有 FOW のインライン FOW だけ)。
fn root() -> Value {
    rrl::column("root", vec![rrl::text("hello", "共有テンプレート(UI は共有 FOW の中)")])
}

// ---------------------------------------------------------------------------
// call
// ---------------------------------------------------------------------------

fn load_template() -> Template {
    host::storage_get(STORAGE_KEY).as_str().and_then(Template::from_id).unwrap_or_default()
}

fn close_and_drop() {
    with_state(|s| {
        if s.open {
            PANEL.close(json!({}));
        }
        *s = State::default();
    });
}

/// 開いている FOW の文面(`token` と `text`)。閉じていれば `None`。
fn current_text() -> Option<(String, String)> {
    with_state(|s| if s.open { Some((s.token.clone(), template_text(s.template, &s.input))) } else { None })
}

fn handle(fn_name: &str, args: Value) -> Result<Value, Value> {
    match fn_name {
        "plugin.manifest" => Ok(manifest(
            PLUGIN_ID,
            "共有テンプレート(サンプル)",
            VERSION,
            "action",
            "投稿をテンプレート(リンクのみ / 引用 / 作者に言及)で整えてコンポーザー・コピー・AsTell へ",
            &["post.share"],
            &["fow.widget", "storage.safe"],
        )),

        // --- 共有 FOW の行を選んだ -------------------------------------------
        "post.share" => {
            let token = str_of(&args, "token");
            if token.is_empty() {
                return Err(err("D01", "token required"));
            }
            let template = load_template();
            let root = with_state(|s| {
                *s = State { token, input: input_from_args(&args), account: account_label(&args), template, open: false };
                panel_root(s)
            });
            let opened = PANEL.open(fow_spec("共有テンプレート", 0.55), root, Some("panelClosed"), json!({}));
            with_state(|s| s.open = opened);
            Ok(json!({}))
        }

        // --- テンプレートの選択 ---------------------------------------------
        f if f.starts_with("tpl.") => {
            let t = Template::from_id(&f["tpl.".len()..]).ok_or_else(|| unsupported(f))?;
            let patch = with_state(|s| {
                if !s.open {
                    return None;
                }
                s.template = t;
                let mut ops: Vec<Value> = Template::ALL
                    .iter()
                    .map(|x| rrl::set_prop(&x.node_key(), "title", json!(tile_title(*x, t))))
                    .collect();
                ops.push(rrl::set_prop("result", "text", json!(template_text(t, &s.input))));
                Some(PANEL.patch(ops))
            });
            let Some(patch) = patch else {
                // 古い FOW からのタップなど: 何もしない
                return Ok(json!({}));
            };
            host::storage_set(STORAGE_KEY, json!(t.id()));
            Ok(patch)
        }

        // --- 共有先 -------------------------------------------------------------
        "share.composer" => {
            if let Some((token, text)) = current_text() {
                host::composer_open(&token, &text, None, None, None);
            }
            Ok(json!({}))
        }
        "share.copy" => {
            if let Some((_, text)) = current_text() {
                host::clipboard_set(&text);
                host::toast("文面をコピーしました");
            }
            Ok(json!({}))
        }
        "share.browser" => {
            let link = with_state(|s| if s.open { Some(s.input.link.clone()) } else { None });
            if let Some(link) = link.filter(|l| !l.is_empty()) {
                host::url_open(&link);
            }
            Ok(json!({}))
        }
        "share.astell" => {
            if let Some((_, text)) = current_text() {
                host::astell_share("text", json!({"text": text}));
                host::toast("AsTell に共有しました");
            }
            Ok(json!({}))
        }

        // --- 共有 FOW が閉じた / FOW がユーザー操作で閉じた / ライフサイクル ------
        "post.share.close" => {
            close_and_drop();
            Ok(json!({}))
        }
        "panelClosed" => {
            with_state(|s| *s = State::default());
            Ok(json!({}))
        }
        "lifecycle" => Ok(json!({})),

        other => Err(unsupported(other)),
    }
}

rasq_plugin! { scope: "sample.post-share:main", root: root, call: handle }

// ---------------------------------------------------------------------------
// テスト(native): ホスト関数はフックで差し替える
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rasq_plugin_kit::{set_host_hook, take_host_log, take_patch_message};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{Mutex as StdMutex, MutexGuard};

    /// テストはグローバル状態を共有するので直列化する。
    static SERIAL: StdMutex<()> = StdMutex::new(());

    const LINK: &str = "https://example.social/@alice/123";

    /// ホストのふり: fow.open は受理、storage.safe は `template` を持つ。
    fn setup(stored: Option<&str>) -> (MutexGuard<'static, ()>, Rc<RefCell<Value>>) {
        let g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let store = Rc::new(RefCell::new(stored.map(|s| json!(s)).unwrap_or(Value::Null)));
        let st = store.clone();
        set_host_hook(move |f, a| match f {
            "fow.open" => json!({"opened": true}),
            "storage.get" if a["key"] == STORAGE_KEY => json!({"value": st.borrow().clone()}),
            "storage.set" if a["key"] == STORAGE_KEY => {
                *st.borrow_mut() = a["value"].clone();
                json!({})
            }
            _ => json!({}),
        });
        with_state(|s| *s = State::default());
        take_host_log();
        take_patch_message();
        (g, store)
    }

    fn share(token: &str, body: &str) -> Result<Value, Value> {
        call_native(
            "post.share",
            json!({
                "token": token,
                "post": {
                    "url": LINK, "sns": "mastodon", "instance": "example.social", "postId": "123",
                    "author": "アリス", "handle": "alice", "body": body, "media": []
                },
                "account": {"sns": "mastodon", "instance": "mine.example", "handle": "me"},
                "link": LINK
            }),
        )
    }

    fn input() -> ShareInput {
        ShareInput { author: "アリス".into(), handle: "alice".into(), instance: "example.social".into(), body: "こんにちは".into(), link: LINK.into() }
    }

    fn find_node<'a>(node: &'a Value, key: &str) -> Option<&'a Value> {
        if node["k"].as_str() == Some(key) {
            return Some(node);
        }
        node["c"].as_array()?.iter().find_map(|c| find_node(c, key))
    }

    fn collect_taps(node: &Value, out: &mut Vec<String>) {
        if let Some(t) = node["on"]["tap"].as_str() {
            out.push(t.to_string());
        }
        if let Some(c) = node["c"].as_array() {
            for ch in c {
                collect_taps(ch, out);
            }
        }
    }

    #[test]
    fn manifest_declares_slot() {
        let m = call_native("plugin.manifest", json!({})).unwrap();
        assert_eq!(m["id"], "sample.post-share");
        assert_eq!(m["kind"], "action");
        assert_eq!(m["slots"], json!(["post.share"]));
        assert_eq!(m["permissions"], json!(["fow.widget", "storage.safe"]));
    }

    #[test]
    fn template_text_shapes() {
        let i = input();
        assert_eq!(template_text(Template::Link, &i), LINK);
        assert_eq!(template_text(Template::Quote, &i), format!("「こんにちは」\n{LINK}"));
        assert_eq!(template_text(Template::Mention, &i), format!("@alice@example.social さんの投稿\n{LINK}"));

        // 80 文字で切って … を付ける(文字数 = Unicode スカラー値。マルチバイトの途中で切れない)
        let long = "あ".repeat(100);
        let q = template_text(Template::Quote, &ShareInput { body: long.clone(), ..i.clone() });
        let expected = format!("「{}…」\n{LINK}", "あ".repeat(QUOTE_CHARS));
        assert_eq!(q, expected);
        // ちょうど 80 文字なら切らない
        let exact = "x".repeat(QUOTE_CHARS);
        let q = template_text(Template::Quote, &ShareInput { body: exact.clone(), ..i.clone() });
        assert_eq!(q, format!("「{exact}」\n{LINK}"));
        // 前後の空白は落とす
        let q = template_text(Template::Quote, &ShareInput { body: "  hi \n".into(), ..i.clone() });
        assert_eq!(q, format!("「hi」\n{LINK}"));

        // ハンドルの形: `@alice` / `alice@host` / `@alice@host` はすべて `@alice@host` に
        assert_eq!(full_handle("@alice", "example.social"), "@alice@example.social");
        assert_eq!(full_handle("alice@other.host", "example.social"), "@alice@other.host");
        assert_eq!(full_handle("@alice@other.host", "example.social"), "@alice@other.host");
        assert_eq!(full_handle("alice", ""), "@alice");
        assert_eq!(truncate_chars("abc", 3), "abc");
        assert_eq!(truncate_chars("abcd", 3), "abc…");
        assert_eq!(Template::from_id("quote"), Some(Template::Quote));
        assert_eq!(Template::from_id("nope"), None);
    }

    #[test]
    fn post_share_opens_fow() {
        let (_g, _store) = setup(None);
        assert_eq!(share("tk1", "こんにちは").unwrap(), json!({}));
        let log = take_host_log();
        assert!(log.iter().any(|(f, a)| f == "storage.get" && a["key"] == STORAGE_KEY));
        let open = log.iter().find(|(f, _)| f == "fow.open").expect("fow.open");
        assert_eq!(open.1["id"], "share-template");
        assert_eq!(open.1["spec"]["title"], "共有テンプレート");
        assert_eq!(open.1["spec"]["initialHeightFactor"], 0.55);
        assert_eq!(open.1["onClosed"], "panelClosed");
        let root = &open.1["root"];
        assert_eq!(find_node(root, "preview-author").unwrap()["p"]["text"], "アリス");
        assert_eq!(find_node(root, "preview-body").unwrap()["p"]["text"], "こんにちは");
        assert_eq!(find_node(root, "account").unwrap()["p"]["text"], "共有するアカウント: @me@mine.example");
        // 既定はリンクのみ(チェック付き)
        assert_eq!(find_node(root, "tpl-link").unwrap()["p"]["title"], "✓ リンクのみ");
        assert_eq!(find_node(root, "tpl-quote").unwrap()["p"]["title"], "　 引用(本文つき)");
        assert_eq!(find_node(root, "tpl-mention").unwrap()["p"]["title"], "　 作者に言及");
        assert_eq!(find_node(root, "result").unwrap()["p"]["text"], LINK);
        let mut taps = vec![];
        collect_taps(root, &mut taps);
        assert_eq!(taps, ["tpl.link", "tpl.quote", "tpl.mention", "share.composer", "share.copy", "share.browser", "share.astell"]);
    }

    #[test]
    fn stored_template_is_preselected() {
        let (_g, _store) = setup(Some("mention"));
        share("tk1", "こんにちは").unwrap();
        let open = take_host_log().into_iter().find(|(f, _)| f == "fow.open").unwrap();
        assert_eq!(find_node(&open.1["root"], "tpl-mention").unwrap()["p"]["title"], "✓ 作者に言及");
        assert_eq!(find_node(&open.1["root"], "result").unwrap()["p"]["text"], format!("@alice@example.social さんの投稿\n{LINK}"));
    }

    #[test]
    fn choose_quote_then_composer() {
        let (_g, store) = setup(None);
        share("tk1", "本文です").unwrap();
        take_host_log();

        // 引用を選ぶ → surface patch(タイトルのチェックと文面)+ storage.set
        let r = call_native("tpl.quote", json!({"nodeId": "sample.post-share:main:tpl-quote", "kind": "tap"})).unwrap();
        assert_eq!(r, json!({"patch": true}));
        let m = take_patch_message();
        assert_eq!(m["surface"], "share-template");
        let ops = m["ops"].as_array().unwrap();
        assert!(ops.iter().any(|o| o["target"] == "sample.post-share:main:tpl-quote" && o["value"] == "✓ 引用(本文つき)"));
        assert!(ops.iter().any(|o| o["target"] == "sample.post-share:main:tpl-link" && o["value"] == "　 リンクのみ"));
        let expected = format!("「本文です」\n{LINK}");
        assert!(ops.iter().any(|o| o["target"] == "sample.post-share:main:result" && o["prop"] == "text" && o["value"] == json!(expected)));
        let log = take_host_log();
        let set = log.iter().find(|(f, _)| f == "storage.set").expect("storage.set");
        assert_eq!(set.1["value"], "quote");
        assert_eq!(*store.borrow(), json!("quote"));

        // コンポーザーへ
        call_native("share.composer", json!({"nodeId": "sample.post-share:main:btn-composer", "kind": "tap"})).unwrap();
        let log = take_host_log();
        let open = log.iter().find(|(f, _)| f == "composer.open").expect("composer.open");
        assert_eq!(open.1, json!({"token": "tk1", "text": expected}));
    }

    #[test]
    fn copy_browser_astell() {
        let (_g, _store) = setup(None);
        share("tk1", "本文").unwrap();
        take_host_log();
        call_native("share.copy", json!({})).unwrap();
        let log = take_host_log();
        assert_eq!(log.iter().find(|(f, _)| f == "clipboard.set").unwrap().1["text"], LINK);
        assert!(log.iter().any(|(f, _)| f == "toast"));
        call_native("share.browser", json!({})).unwrap();
        assert_eq!(take_host_log().iter().find(|(f, _)| f == "url.open").unwrap().1["url"], LINK);
        call_native("tpl.mention", json!({})).unwrap();
        take_host_log();
        call_native("share.astell", json!({})).unwrap();
        let s = take_host_log().into_iter().find(|(f, _)| f == "astell.share").unwrap();
        assert_eq!(s.1, json!({"kind": "text", "payload": {"text": format!("@alice@example.social さんの投稿\n{LINK}")}}));
    }

    #[test]
    fn close_drops_state() {
        let (_g, _store) = setup(None);
        share("tk1", "本文").unwrap();
        take_host_log();
        call_native("post.share.close", json!({"token": "tk1"})).unwrap();
        let log = take_host_log();
        assert!(log.iter().any(|(f, a)| f == "fow.close" && a["id"] == "share-template"));
        // 閉じた後はボタンもテンプレートも何もしない
        call_native("share.composer", json!({})).unwrap();
        call_native("tpl.quote", json!({})).unwrap();
        assert!(take_host_log().is_empty());
        // もう一度 close しても fow.close は呼ばない
        call_native("post.share.close", json!({"token": "tk1"})).unwrap();
        assert!(take_host_log().is_empty());

        // ユーザーが ✕ で閉じた場合は状態だけ捨てる(fow.close は呼ばない)
        share("tk2", "本文").unwrap();
        call_native("panelClosed", json!({"id": "share-template", "reason": "cancel"})).unwrap();
        take_host_log();
        call_native("share.copy", json!({})).unwrap();
        assert!(take_host_log().is_empty());
    }

    #[test]
    fn bad_input_does_not_panic() {
        let (_g, _store) = setup(None);
        assert!(call_native("post.share", json!(null)).is_err());
        assert!(call_native("post.share", json!({"token": "t", "post": 5})).is_ok());
        assert!(call_native("tpl.nope", json!({})).is_err());
        assert!(call_native("lifecycle", json!({"kind": "mount"})).is_ok());
        assert!(call_native("nope", json!({})).is_err());
    }
}
