//! 統合テスト・E2E 用のテストランナー（[libtest-mimic] で標準のテストハーネスを置き換える）。
//!
//! テスト名を識別子に変換せず、書いた文字列のまま `describe` の名前と ` > ` でつないで表示する。
//!
//! ```ignore
//! // Cargo.toml: [[test]] name = "api" / harness = false
//! use meetup_test_runner::{Node, describe, run, test};
//!
//! fn main() {
//!     run(vec![認証()]);
//! }
//!
//! fn 認証() -> Node {
//!     describe("認証", vec![
//!         describe("新規登録", vec![
//!             test("同じメールアドレスの登録は 409 になる", || async {
//!                 // ...
//!             }),
//!         ]),
//!     ])
//! }
//! // cargo test の表示: 認証 > 新規登録 > 同じメールアドレスの登録は 409 になる
//! ```
//!
//! - テスト本体は `|| async { … }`。テストごとに tokio のランタイムを作って実行する（既定は current_thread）
//! - `.multi_thread()` / `.ignore(理由)` は `test` にも `describe` にも付けられる（`describe` なら配下の全テストに効く）
//! - 絞り込み（`cargo test --test api 新規登録`）、`--ignored`、`--list` などは標準のハーネスと同じ
//! - フルネーム（` > ` でつないだ名前）が重なったら、実行前に panic する
//!
//! [libtest-mimic]: https://crates.io/crates/libtest-mimic

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use libtest_mimic::{Arguments, Failed, Trial};

type Body = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()>>> + Send>;

/// `describe` / `test` の木の 1 要素
pub struct Node {
    kind: Kind,
    multi_thread: bool,
    ignore: Option<String>,
}

enum Kind {
    Group { name: String, children: Vec<Node> },
    Case { name: String, body: Body },
}

/// テストをまとめる。名前は配下のテスト名の前に ` > ` でつながる
pub fn describe(name: &str, children: Vec<Node>) -> Node {
    Node::new(Kind::Group {
        name: name.to_string(),
        children,
    })
}

/// 1 件のテスト。本体は `|| async { … }`
pub fn test<F, Fut>(name: &str, body: F) -> Node
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + 'static,
{
    Node::new(Kind::Case {
        name: name.to_string(),
        body: Box::new(move || Box::pin(body())),
    })
}

impl Node {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            multi_thread: false,
            ignore: None,
        }
    }

    /// multi_thread のランタイムで実行する（`#[tokio::test(flavor = "multi_thread")]` 相当）
    pub fn multi_thread(mut self) -> Self {
        self.multi_thread = true;
        self
    }

    /// 既定では実行せず、`--ignored` を付けたときだけ実行する（`#[ignore = "…"]` 相当）
    pub fn ignore(mut self, reason: &str) -> Self {
        self.ignore = Some(reason.to_string());
        self
    }

    fn flatten(self, prefix: &str, multi_thread: bool, ignored: bool, out: &mut Vec<Trial>) {
        let multi_thread = multi_thread || self.multi_thread;
        let ignored = ignored || self.ignore.is_some();
        match self.kind {
            Kind::Group { name, children } => {
                let prefix = join(prefix, &name);
                for child in children {
                    child.flatten(&prefix, multi_thread, ignored, out);
                }
            }
            Kind::Case { name, body } => {
                let trial = Trial::test(join(prefix, &name), move || execute(body, multi_thread));
                out.push(trial.with_ignored_flag(ignored));
            }
        }
    }
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix} > {name}")
    }
}

/// テストごとにランタイムを作り、本体を最後まで実行する（panic は libtest-mimic が失敗として扱う）
fn execute(body: Body, multi_thread: bool) -> Result<(), Failed> {
    let mut builder = if multi_thread {
        tokio::runtime::Builder::new_multi_thread()
    } else {
        tokio::runtime::Builder::new_current_thread()
    };
    let runtime = builder
        .enable_all()
        .build()
        .map_err(|e| Failed::from(format!("tokio のランタイムを作れませんでした: {e}")))?;
    runtime.block_on(body());
    Ok(())
}

/// テストの木を平らにして実行し、結果に応じた終了コードでプロセスを終える
pub fn run(tests: Vec<Node>) -> ! {
    let args = Arguments::from_args();
    let trials = collect(tests);
    libtest_mimic::run(&args, trials).exit()
}

/// フルネームの一覧に平らにする（重複していれば panic）
fn collect(tests: Vec<Node>) -> Vec<Trial> {
    let mut trials = Vec::new();
    for node in tests {
        node.flatten("", false, false, &mut trials);
    }
    let mut seen = HashSet::new();
    for trial in &trials {
        assert!(
            seen.insert(trial.name().to_string()),
            "テスト名が重複しています: {}",
            trial.name()
        );
    }
    trials
}

#[cfg(test)]
mod tests {
    use super::*;
    use meetup_test_macros::describe as describe_macro;

    fn names(nodes: Vec<Node>) -> Vec<(String, bool)> {
        collect(nodes)
            .iter()
            .map(|t| (t.name().to_string(), t.has_ignored_flag()))
            .collect()
    }

    describe_macro!("テストランナー", {
        test!(
            "describe の名前を > でつないだフルネームになる",
            {
                let tree = vec![describe(
                    "認証",
                    vec![describe("新規登録", vec![test("409 になる", || async {})])],
                )];
                assert_eq!(
                    names(tree),
                    vec![("認証 > 新規登録 > 409 になる".to_string(), false)]
                );
            }
        );

        test!("describe の ignore は配下の全テストに効く", {
            let tree = vec![
                describe("E2E", vec![test("a", || async {}), test("b", || async {})])
                    .ignore("ブラウザが必要"),
                test("c", || async {}),
            ];
            let ignored: Vec<bool> = names(tree).into_iter().map(|(_, i)| i).collect();
            assert_eq!(ignored, vec![true, true, false]);
        });

        test!("本体の async を最後まで実行する", {
            let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let seen = flag.clone();
            let body: Body = Box::new(move || {
                Box::pin(async move {
                    tokio::task::yield_now().await;
                    seen.store(true, std::sync::atomic::Ordering::SeqCst);
                })
            });
            assert!(execute(body, true).is_ok());
            assert!(flag.load(std::sync::atomic::Ordering::SeqCst));
        });

        #[should_panic(expected = "テスト名が重複しています")]
        test!("フルネームが重複したら panic する", {
            let tree = vec![test("同じ", || async {}), test("同じ", || async {})];
            collect(tree);
        });
    });
}
