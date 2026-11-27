//! `shared/middleware.rs` の単体テスト

use super::*;
use meetup_test_macros::describe;

fn request_from_lines(lines: &[&str]) -> Request {
    let mut builder = Request::builder();
    for v in lines {
        builder = builder.header("x-forwarded-for", *v);
    }
    let mut req = builder.body(axum::body::Body::empty()).unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([10, 0, 0, 1], 1234))));
    req
}

describe!("ミドルウェア", {
    describe!("キャッシュ", {
        test!("公開 GET パスを判定できる", {
            assert!(is_public_read_path("/communities"));
            assert!(is_public_read_path("/communities/abc"));
            assert!(is_public_read_path("/communities/abc/members"));
            assert!(is_public_read_path("/communities/abc/events"));
            assert!(!is_public_read_path("/auth/me"));
            assert!(!is_public_read_path("/communities/abc/members/me"));
        });
    });

    describe!("レート制限", {
        test!("上限を超えると拒否する", {
            let limiter = RateLimiter::new(true);
            assert!((0..3).all(|_| limiter.check("t", "1.1.1.1", 3)));
            assert!(!limiter.check("t", "1.1.1.1", 3));
            assert!(limiter.check("t", "2.2.2.2", 3));
        });

        test!(
            "期限切れのバケットは掃除されメモリが増え続けない",
            {
                let limiter = RateLimiter::new(true);
                {
                    // 期限切れのバケットを閾値ぶん仕込む
                    let mut buckets = limiter.buckets.lock().unwrap();
                    let expired = Instant::now() - WINDOW - Duration::from_secs(1);
                    for i in 0..EVICT_THRESHOLD {
                        buckets
                            .insert(("t", format!("10.0.{}.{}", i / 256, i % 256)), (expired, 1));
                    }
                }
                assert!(limiter.check("t", "9.9.9.9", 3));
                let len = limiter.buckets.lock().unwrap().len();
                assert_eq!(len, 1, "期限切れが掃除され、新しい 1 件だけが残る");
            }
        );

        test!("無効時は常に許可する", {
            let limiter = RateLimiter::new(false);
            assert!((0..1000).all(|_| limiter.check("t", "1.1.1.1", 1)));
        });
    });

    describe!("クライアント IP の特定", {
        test!(
            "信頼するプロキシの末尾の IP を使い、不正な値は接続元 IP にする",
            {
                let req = request_from_lines(&["198.51.100.1, 203.0.113.7"]);
                assert_eq!(client_ip(&req, true), "203.0.113.7");
                assert_eq!(client_ip(&req, false), "10.0.0.1");
                let forged = request_from_lines(&["198.51.100.1, not-an-ip"]);
                assert_eq!(client_ip(&forged, true), "10.0.0.1");
            }
        );

        test!("複数行の X-Forwarded-For は最後の行を使う", {
            // クライアントが偽装した行の後に、信頼するプロキシが行を追加する構成
            let req = request_from_lines(&["198.51.100.99", "203.0.113.7"]);
            assert_eq!(client_ip(&req, true), "203.0.113.7");
        });

        test!(
            "ポート付きの X-Forwarded-For も IP として扱う",
            {
                assert_eq!(
                    client_ip(&request_from_lines(&["203.0.113.7:51234"]), true),
                    "203.0.113.7"
                );
                assert_eq!(
                    client_ip(&request_from_lines(&["[2001:db8:1:2::5]:443"]), true),
                    "2001:db8:1:2::/64"
                );
            }
        );

        test!("IPv6 は 64 ビット単位でまとめる", {
            let a: IpAddr = "2001:db8:1:2:aaaa::1".parse().unwrap();
            let b: IpAddr = "2001:db8:1:2:bbbb::2".parse().unwrap();
            assert_eq!(rate_limit_key(a), rate_limit_key(b));
            let other: IpAddr = "2001:db8:1:3::1".parse().unwrap();
            assert_ne!(rate_limit_key(a), rate_limit_key(other));
        });

        test!(
            "IPv4 はそのまま、IPv4 射影アドレスは IPv4 として扱う",
            {
                let v4: IpAddr = "192.0.2.1".parse().unwrap();
                let mapped: IpAddr = "::ffff:192.0.2.1".parse().unwrap();
                assert_eq!(rate_limit_key(v4), "192.0.2.1");
                assert_eq!(rate_limit_key(mapped), "192.0.2.1");
            }
        );
    });
});
