use axum::http::StatusCode;
use meetup_test_runner::{Node, describe, run, test};
use serde_json::Value;

mod common;
use common::*;

// 業務 API のうちメンバー（参加・脱退・承認・拒否・メンバー一覧）。api.rs が 800 行に近づいたため分けた
fn main() {
    run(vec![メンバー()]);
}

fn メンバー() -> Node {
    describe(
        "メンバー",
        vec![
            test(
                "公開コミュニティへの参加は ACTIVE、重複参加は 409、脱退できる",
                || async {
                    let app = app().await;
                    let owner = register(&app, "j-owner").await;
                    let user = register(&app, "j-user").await;
                    let id = create_community(&app, &owner, "open", "PUBLIC").await;
                    let members = format!("/communities/{id}/members");

                    let (status, body) =
                        call(&app, "POST", &members, Some(&user.token), None).await;
                    assert_eq!(status, StatusCode::CREATED);
                    assert_eq!(body["status"], "ACTIVE");
                    assert_eq!(body["role"], "MEMBER");

                    let (status, body) =
                        call(&app, "POST", &members, Some(&user.token), None).await;
                    assert_eq!(status, StatusCode::CONFLICT);
                    assert_eq!(body["code"], "ALREADY_MEMBER");

                    let (status, _) = call(
                        &app,
                        "DELETE",
                        &format!("{members}/me"),
                        Some(&user.token),
                        None,
                    )
                    .await;
                    assert_eq!(status, StatusCode::OK);
                    let (status, body) = call(
                        &app,
                        "DELETE",
                        &format!("{members}/me"),
                        Some(&user.token),
                        None,
                    )
                    .await;
                    assert_eq!(status, StatusCode::NOT_FOUND);
                    assert_eq!(body["code"], "MEMBER_NOT_FOUND");
                },
            ),
            test("オーナーは脱退できない", || async {
                let app = app().await;
                let owner = register(&app, "ol-owner").await;
                let id = create_community(&app, &owner, "no-leave", "PUBLIC").await;
                let (status, body) = call(
                    &app,
                    "DELETE",
                    &format!("/communities/{id}/members/me"),
                    Some(&owner.token),
                    None,
                )
                .await;
                assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
                assert_eq!(body["code"], "OWNER_CANNOT_LEAVE");
            }),
            test(
                "メンバー ID 指定の脱退は本人のメンバー ID のみ有効",
                || async {
                    let app = app().await;
                    let owner = register(&app, "id-owner").await;
                    let user = register(&app, "id-user").await;
                    let other = register(&app, "id-other").await;
                    let id = create_community(&app, &owner, "by-id", "PUBLIC").await;
                    let members = format!("/communities/{id}/members");
                    let (_, joined) = call(&app, "POST", &members, Some(&user.token), None).await;
                    let member_id = joined["id"].as_str().unwrap();
                    call(&app, "POST", &members, Some(&other.token), None).await;

                    let (status, _) = call(
                        &app,
                        "DELETE",
                        &format!("{members}/{member_id}"),
                        Some(&other.token),
                        None,
                    )
                    .await;
                    assert_eq!(
                        status,
                        StatusCode::NOT_FOUND,
                        "他人のメンバーIDでは脱退できない"
                    );
                    let (status, _) = call(
                        &app,
                        "DELETE",
                        &format!("{members}/{member_id}"),
                        Some(&user.token),
                        None,
                    )
                    .await;
                    assert_eq!(status, StatusCode::OK);
                },
            ),
            test(
                "非公開コミュニティは承認制で、承認と拒否ができる",
                || async {
                    let app = app().await;
                    let owner = register(&app, "ap-owner").await;
                    let u1 = register(&app, "ap-u1").await;
                    let u2 = register(&app, "ap-u2").await;
                    let id = create_community(&app, &owner, "approval", "PRIVATE").await;
                    let members = format!("/communities/{id}/members");

                    let (_, j1) = call(&app, "POST", &members, Some(&u1.token), None).await;
                    let (_, j2) = call(&app, "POST", &members, Some(&u2.token), None).await;
                    assert_eq!(j1["status"], "PENDING");
                    let m1 = j1["id"].as_str().unwrap();
                    let m2 = j2["id"].as_str().unwrap();

                    // 一般メンバーは承認できない（403）
                    let (status, body) = call(
                        &app,
                        "PATCH",
                        &format!("{members}/{m1}/approve"),
                        Some(&u2.token),
                        None,
                    )
                    .await;
                    assert_eq!(status, StatusCode::FORBIDDEN);
                    assert_eq!(body["code"], "FORBIDDEN");

                    let (status, body) = call(
                        &app,
                        "PATCH",
                        &format!("{members}/{m1}/approve"),
                        Some(&owner.token),
                        None,
                    )
                    .await;
                    assert_eq!(status, StatusCode::OK);
                    assert_eq!(body["status"], "ACTIVE");

                    let (status, body) = call(
                        &app,
                        "PATCH",
                        &format!("{members}/{m1}/approve"),
                        Some(&owner.token),
                        None,
                    )
                    .await;
                    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
                    assert_eq!(body["code"], "MEMBER_ALREADY_ACTIVE");

                    let (status, _) = call(
                        &app,
                        "PATCH",
                        &format!("{members}/{m2}/reject"),
                        Some(&owner.token),
                        None,
                    )
                    .await;
                    assert_eq!(status, StatusCode::NO_CONTENT);
                },
            ),
            test(
                "別コミュニティのメンバーは承認できない",
                || async {
                    let app = app().await;
                    let owner_a = register(&app, "x-owner-a").await;
                    let owner_b = register(&app, "x-owner-b").await;
                    let user = register(&app, "x-user").await;
                    let a = create_community(&app, &owner_a, "comm-a", "PRIVATE").await;
                    let b = create_community(&app, &owner_b, "comm-b", "PRIVATE").await;
                    let (_, joined) = call(
                        &app,
                        "POST",
                        &format!("/communities/{b}/members"),
                        Some(&user.token),
                        None,
                    )
                    .await;
                    let member_id = joined["id"].as_str().unwrap();

                    let (status, body) = call(
                        &app,
                        "PATCH",
                        &format!("/communities/{a}/members/{member_id}/approve"),
                        Some(&owner_a.token),
                        None,
                    )
                    .await;
                    assert_eq!(status, StatusCode::NOT_FOUND);
                    assert_eq!(body["code"], "MEMBER_NOT_FOUND");
                },
            ),
            test(
                "メンバー一覧の accountId は ACTIVE メンバーにだけ見える",
                || async {
                    let app = app().await;
                    let owner = register(&app, "ls-owner").await;
                    let outsider = register(&app, "ls-out").await;
                    let id = create_community(&app, &owner, "listing", "PUBLIC").await;
                    let path = format!("/communities/{id}/members");

                    let (status, body) = call(&app, "GET", &path, None, None).await;
                    assert_eq!(status, StatusCode::OK);
                    assert_eq!(body["total"], 1);
                    assert_eq!(body["members"][0]["accountName"], "ls-owner");
                    assert!(body["members"][0].get("accountId").is_none());

                    let (_, body) = call(&app, "GET", &path, Some(&outsider.token), None).await;
                    assert!(body["members"][0].get("accountId").is_none());

                    let (_, body) = call(&app, "GET", &path, Some(&owner.token), None).await;
                    assert_eq!(body["members"][0]["accountId"], owner.id);
                },
            ),
            test(
                "非公開コミュニティのメンバー一覧は非メンバーに 404",
                || async {
                    let app = app().await;
                    let owner = register(&app, "pl-owner").await;
                    let id = create_community(&app, &owner, "private-list", "PRIVATE").await;
                    let path = format!("/communities/{id}/members");
                    assert_eq!(
                        call(&app, "GET", &path, None, None).await.0,
                        StatusCode::NOT_FOUND
                    );
                    assert_eq!(
                        call(&app, "GET", &path, Some(&owner.token), None).await.0,
                        StatusCode::OK
                    );
                },
            ),
            test(
                "オーナーのメンバー一覧には承認待ちの申請者が含まれる",
                || async {
                    let app = app().await;
                    let owner = register(&app, "pd-owner").await;
                    let applicant = register(&app, "pd-applicant").await;
                    let id = create_community(&app, &owner, "pending-owner", "PRIVATE").await;
                    let path = format!("/communities/{id}/members");
                    call(&app, "POST", &path, Some(&applicant.token), None).await;

                    let (status, body) = call(&app, "GET", &path, Some(&owner.token), None).await;
                    assert_eq!(status, StatusCode::OK);
                    assert_eq!(body["total"], 2);
                    let members = body["members"].as_array().unwrap();
                    assert_eq!(members.len(), 2);
                    let pending: Vec<&Value> = members
                        .iter()
                        .filter(|m| m["status"] == "PENDING")
                        .collect();
                    assert_eq!(pending.len(), 1);
                    assert_eq!(pending[0]["accountName"], "pd-applicant");
                },
            ),
            test(
                "管理者のメンバー一覧には承認待ちの申請者が含まれる",
                || async {
                    let (app, pool) = app_with_pool().await;
                    let owner = register(&app, "pa-owner").await;
                    let admin = register(&app, "pa-admin").await;
                    let applicant = register(&app, "pa-applicant").await;
                    let id = create_community(&app, &owner, "pending-admin", "PRIVATE").await;
                    let path = format!("/communities/{id}/members");
                    let (_, admin_join) = call(&app, "POST", &path, Some(&admin.token), None).await;
                    // ADMIN を付与する API は無いため、DB で昇格させる
                    sqlx::query("UPDATE community_members SET role = 'ADMIN', status = 'ACTIVE' WHERE id = ?")
                .bind(admin_join["id"].as_str().unwrap())
                .execute(&pool)
                .await
                .unwrap();
                    call(&app, "POST", &path, Some(&applicant.token), None).await;

                    let (status, body) = call(&app, "GET", &path, Some(&admin.token), None).await;
                    assert_eq!(status, StatusCode::OK);
                    assert_eq!(body["total"], 3);
                    let members = body["members"].as_array().unwrap();
                    assert_eq!(members.len(), 3);
                    assert!(
                members
                    .iter()
                    .any(|m| m["status"] == "PENDING" && m["accountName"] == "pa-applicant")
            );
                },
            ),
            test(
                "承認待ちの申請者はメンバー一覧の先頭に並ぶ",
                || async {
                    let app = app().await;
                    let owner = register(&app, "po-owner").await;
                    let applicant = register(&app, "po-applicant").await;
                    let id = create_community(&app, &owner, "pending-order", "PRIVATE").await;
                    let path = format!("/communities/{id}/members");
                    call(&app, "POST", &path, Some(&applicant.token), None).await;

                    // 申請はオーナーより後に作られるが、1 件目のページに入る
                    let (status, body) = call(
                        &app,
                        "GET",
                        &format!("{path}?limit=1"),
                        Some(&owner.token),
                        None,
                    )
                    .await;
                    assert_eq!(status, StatusCode::OK);
                    assert_eq!(body["total"], 2);
                    assert_eq!(body["members"][0]["status"], "PENDING");
                    assert_eq!(body["members"][0]["accountName"], "po-applicant");
                },
            ),
            test(
                "承認待ちの申請者本人には非公開コミュニティのメンバー一覧が 404",
                || async {
                    let app = app().await;
                    let owner = register(&app, "ps-owner").await;
                    let applicant = register(&app, "ps-applicant").await;
                    let id = create_community(&app, &owner, "pending-self", "PRIVATE").await;
                    let path = format!("/communities/{id}/members");
                    call(&app, "POST", &path, Some(&applicant.token), None).await;

                    let (status, _) = call(&app, "GET", &path, Some(&applicant.token), None).await;
                    assert_eq!(status, StatusCode::NOT_FOUND);
                },
            ),
            test(
                "一般メンバーのメンバー一覧には承認待ちの申請者が含まれない",
                || async {
                    let app = app().await;
                    let owner = register(&app, "pm-owner").await;
                    let member = register(&app, "pm-member").await;
                    let applicant = register(&app, "pm-applicant").await;
                    let id = create_community(&app, &owner, "pending-member", "PRIVATE").await;
                    let path = format!("/communities/{id}/members");
                    let (_, joined) = call(&app, "POST", &path, Some(&member.token), None).await;
                    let member_id = joined["id"].as_str().unwrap();
                    call(
                        &app,
                        "PATCH",
                        &format!("{path}/{member_id}/approve"),
                        Some(&owner.token),
                        None,
                    )
                    .await;
                    call(&app, "POST", &path, Some(&applicant.token), None).await;

                    let (status, body) = call(&app, "GET", &path, Some(&member.token), None).await;
                    assert_eq!(status, StatusCode::OK);
                    assert_eq!(body["total"], 2);
                    let members = body["members"].as_array().unwrap();
                    assert_eq!(members.len(), 2);
                    assert!(members.iter().all(|m| m["status"] == "ACTIVE"));
                },
            ),
        ],
    )
}
