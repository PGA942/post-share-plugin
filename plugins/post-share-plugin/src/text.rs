//! 利用者に見せる文の表(PROTOCOL.md §4.6)。ホストの `args.locale`(無ければ `ja`)で引く。
//!
//! 基準は `ja`。`en` / `zh-Hans` / `zh-Hant` / `ko` は同じキーを全部持つ(テストで確かめる)。
//! `tpl.*.text` は共有する文面そのもの(利用者の言語で組む)。

rasq_plugin_kit::i18n_catalog! {
    pub(crate) static TEXT = {
        "ja": {
            "root.hello" => "共有テンプレート(UI は共有 FOW の中)",
            "panel.title" => "共有テンプレート",
            "panel.account" => "共有するアカウント: {account}",
            "panel.template" => "テンプレート",
            "panel.result" => "文面",
            "button.composer" => "コンポーザーへ",
            "button.copy" => "コピー",
            "button.browser" => "ブラウザで開く",
            "button.astell" => "ASTell へ",
            "tpl.link.label" => "リンクのみ",
            "tpl.quote.label" => "引用(本文つき)",
            "tpl.mention.label" => "作者に言及",
            "tpl.quote.text" => "「{body}」\n{link}",
            "tpl.mention.text" => "{handle} さんの投稿\n{link}",
            "toast.copied" => "文面をコピーしました",
            "toast.astell" => "ASTell に共有しました",
        },
        "en": {
            "root.hello" => "Share templates (the UI is inside the share panel)",
            "panel.title" => "Share templates",
            "panel.account" => "Sharing as: {account}",
            "panel.template" => "Template",
            "panel.result" => "Text",
            "button.composer" => "Open in composer",
            "button.copy" => "Copy",
            "button.browser" => "Open in browser",
            "button.astell" => "Send to ASTell",
            "tpl.link.label" => "Link only",
            "tpl.quote.label" => "Quote (with text)",
            "tpl.mention.label" => "Mention the author",
            "tpl.quote.text" => "“{body}”\n{link}",
            "tpl.mention.text" => "Post by {handle}\n{link}",
            "toast.copied" => "Text copied",
            "toast.astell" => "Shared to ASTell",
        },
        "zh-Hans": {
            "root.hello" => "分享模板（界面在分享面板中）",
            "panel.title" => "分享模板",
            "panel.account" => "分享所用账号：{account}",
            "panel.template" => "模板",
            "panel.result" => "分享文字",
            "button.composer" => "在编辑器中打开",
            "button.copy" => "复制",
            "button.browser" => "在浏览器中打开",
            "button.astell" => "发送到 ASTell",
            "tpl.link.label" => "仅链接",
            "tpl.quote.label" => "引用（含正文）",
            "tpl.mention.label" => "提及作者",
            "tpl.quote.text" => "“{body}”\n{link}",
            "tpl.mention.text" => "{handle} 的帖子\n{link}",
            "toast.copied" => "已复制分享文字",
            "toast.astell" => "已分享到 ASTell",
        },
        "zh-Hant": {
            "root.hello" => "分享範本（介面在分享面板中）",
            "panel.title" => "分享範本",
            "panel.account" => "分享所用帳號：{account}",
            "panel.template" => "範本",
            "panel.result" => "分享文字",
            "button.composer" => "在編輯器中開啟",
            "button.copy" => "複製",
            "button.browser" => "在瀏覽器中開啟",
            "button.astell" => "傳送到 ASTell",
            "tpl.link.label" => "僅連結",
            "tpl.quote.label" => "引用（含內文）",
            "tpl.mention.label" => "提及作者",
            "tpl.quote.text" => "「{body}」\n{link}",
            "tpl.mention.text" => "{handle} 的貼文\n{link}",
            "toast.copied" => "已複製分享文字",
            "toast.astell" => "已分享到 ASTell",
        },
        "ko": {
            "root.hello" => "공유 템플릿(UI는 공유 패널 안에 있습니다)",
            "panel.title" => "공유 템플릿",
            "panel.account" => "공유할 계정: {account}",
            "panel.template" => "템플릿",
            "panel.result" => "공유 문구",
            "button.composer" => "작성 화면에서 열기",
            "button.copy" => "복사",
            "button.browser" => "브라우저에서 열기",
            "button.astell" => "ASTell로 보내기",
            "tpl.link.label" => "링크만",
            "tpl.quote.label" => "인용(본문 포함)",
            "tpl.mention.label" => "작성자 언급",
            "tpl.quote.text" => "“{body}”\n{link}",
            "tpl.mention.text" => "{handle} 님의 게시물\n{link}",
            "toast.copied" => "공유 문구가 복사되었습니다",
            "toast.astell" => "ASTell에 공유되었습니다",
        },
    };
}

#[cfg(test)]
mod tests {
    use super::TEXT;
    use std::collections::BTreeSet;

    /// この表の言語ごとのキー(`Catalog::check` は「基準に無いキー」と差し込みの食い違いしか見ないので、訳の抜けはここで見る)。
    fn table_keys() -> Vec<(String, BTreeSet<String>)> {
        let mut out: Vec<(String, BTreeSet<String>)> = Vec::new();
        for line in include_str!("text.rs").lines().map(str::trim) {
            if let Some(tag) = line.strip_prefix('"').and_then(|l| l.strip_suffix("\": {")) {
                out.push((tag.to_string(), BTreeSet::new()));
            } else if let Some((key, _)) = line.strip_prefix('"').and_then(|l| l.split_once("\" =>")) {
                out.last_mut().expect("表の外のキー").1.insert(key.to_string());
            }
        }
        out
    }

    #[test]
    fn every_locale_has_every_key() {
        assert!(TEXT.check().is_empty(), "{:?}", TEXT.check());
        let tables = table_keys();
        let tags: Vec<&str> = tables.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(tags, TEXT.locales());
        assert_eq!(tags, ["ja", "en", "zh-Hans", "zh-Hant", "ko"]);
        for (tag, keys) in &tables[1..] {
            let missing: Vec<_> = tables[0].1.difference(keys).collect();
            assert!(missing.is_empty(), "{tag} に訳が無いキー: {missing:?}");
        }
    }
}
