//! describe! / test! 自身のテスト。変換後の識別子は、生成された mod の関数を名前で呼べるか（コンパイルが通るか）で確かめる

use meetup_test_macros::describe;

describe!("識別子への変換", {
    describe!("新規登録できる", {
        pub fn reachable() {}
    });
    describe!("同名は 409、上限は 422（作成）", {
        pub fn reachable() {}
    });
    describe!("/auth/me は認証が必要", {
        pub fn reachable() {}
    });
    describe!("404 を返す", {
        pub fn reachable() {}
    });
    describe!("match", {
        pub fn reachable() {}
    });
    describe!("gen", {
        pub fn reachable() {}
    });

    test!("日本語はそのまま識別子になる", {
        新規登録できる::reachable();
    });

    test!(
        "空白や記号の並びは 1 つのアンダースコアにまとめ、末尾の記号は削る",
        {
            同名は_409_上限は_422_作成::reachable();
        }
    );

    test!("先頭の記号は削る", {
        auth_me_は認証が必要::reachable();
    });

    test!(
        "数字で始まる名前は先頭にアンダースコアを付ける",
        {
            _404_を返す::reachable();
        }
    );

    test!(
        "キーワードと衝突する名前は末尾にアンダースコアを付ける",
        {
            match_::reachable();
            gen_::reachable();
        }
    );
});

describe!("テスト関数の生成", {
    test!("ブロックの本体は同期のテストになる", {
        assert_eq!(1 + 1, 2);
    });

    test!("async の本体は tokio のテストになる", async {
        tokio::task::yield_now().await;
    });

    #[tokio::test(flavor = "multi_thread")]
    test!(
        "テスト属性を書けば既定の属性の代わりに使う",
        async {
            let flavor = tokio::runtime::Handle::current().runtime_flavor();
            assert_eq!(flavor, tokio::runtime::RuntimeFlavor::MultiThread);
        }
    );

    #[ignore = "ignore 属性がそのまま付くことの確認用"]
    test!("その他の属性はそのまま付く", {
        unreachable!("ignore されるので実行されない");
    });

    fn helper() -> u8 {
        42
    }

    test!("describe の中のヘルパーを使える", {
        assert_eq!(helper(), 42);
    });
});
