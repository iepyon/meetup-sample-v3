use serde_json::{Value, json};

// ============================================================
// OpenAPI ドキュメント（`/openapi.json`）と Swagger UI（`/docs`）
// ============================================================

fn error_ref() -> Value {
    json!({ "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } })
}

fn resp(description: &str) -> Value {
    let mut v = error_ref();
    v["description"] = json!(description);
    v
}

fn id_param(name: &str) -> Value {
    json!({ "name": name, "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } })
}

fn page_params() -> Vec<Value> {
    vec![
        json!({ "name": "limit", "in": "query", "schema": { "type": "integer", "minimum": 0, "maximum": 100, "default": 20 } }),
        json!({ "name": "offset", "in": "query", "schema": { "type": "integer", "minimum": 0, "default": 0 } }),
    ]
}

fn json_body(schema: &str) -> Value {
    json!({ "required": true, "content": { "application/json": { "schema": { "$ref": format!("#/components/schemas/{schema}") } } } })
}

fn ok_json(description: &str, schema: &str) -> Value {
    json!({ "description": description, "content": { "application/json": { "schema": { "$ref": format!("#/components/schemas/{schema}") } } } })
}

pub fn document() -> Value {
    let bearer = json!([{ "BearerAuth": [] }]);
    let mut list_communities_params = vec![
        json!({ "name": "category", "in": "query", "schema": { "type": "string", "enum": ["TECH", "BUSINESS", "HOBBY"] } }),
        json!({ "name": "member", "in": "query", "schema": { "type": "string", "enum": ["me"] } }),
    ];
    list_communities_params.extend(page_params());

    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Meetup Community Management API",
            "description": "Backend API for meetup community management",
            "version": "0.1.0"
        },
        "servers": [{ "url": "http://localhost:3000", "description": "Development server" }],
        "tags": [
            { "name": "Auth", "description": "Authentication" },
            { "name": "Communities", "description": "Community management" },
            { "name": "Members", "description": "Community member management" },
            { "name": "Events", "description": "Event management" },
            { "name": "System", "description": "System" }
        ],
        "components": {
            "securitySchemes": { "BearerAuth": { "type": "http", "scheme": "bearer", "bearerFormat": "JWT" } },
            "schemas": {
                "ErrorResponse": { "type": "object", "required": ["code", "message"],
                    "properties": { "code": { "type": "string", "example": "NOT_FOUND" }, "message": { "type": "string" } } },
                "Account": { "type": "object", "required": ["id", "name", "email", "createdAt"],
                    "properties": { "id": { "type": "string", "format": "uuid" }, "name": { "type": "string" },
                        "email": { "type": "string", "format": "email" }, "createdAt": { "type": "string", "format": "date-time" } } },
                "AuthResponse": { "type": "object", "required": ["token", "account"],
                    "properties": { "token": { "type": "string" }, "account": { "$ref": "#/components/schemas/Account" } } },
                "RegisterRequest": { "type": "object", "required": ["name", "email", "password"],
                    "properties": { "name": { "type": "string", "minLength": 1, "maxLength": 100 },
                        "email": { "type": "string", "format": "email" }, "password": { "type": "string", "minLength": 1 } } },
                "LoginRequest": { "type": "object", "required": ["email", "password"],
                    "properties": { "email": { "type": "string", "format": "email" }, "password": { "type": "string" } } },
                "Community": { "type": "object",
                    "required": ["id", "name", "description", "category", "visibility", "createdAt", "updatedAt"],
                    "properties": { "id": { "type": "string", "format": "uuid" }, "name": { "type": "string", "minLength": 1, "maxLength": 100 },
                        "description": { "type": "string", "nullable": true, "maxLength": 1000 },
                        "category": { "type": "string", "enum": ["TECH", "BUSINESS", "HOBBY"] },
                        "visibility": { "type": "string", "enum": ["PUBLIC", "PRIVATE"] },
                        "createdAt": { "type": "string", "format": "date-time" }, "updatedAt": { "type": "string", "format": "date-time" } } },
                "CommunityResponse": { "type": "object", "properties": { "community": { "$ref": "#/components/schemas/Community" } } },
                "ListCommunitiesResponse": { "type": "object",
                    "properties": { "communities": { "type": "array", "items": { "$ref": "#/components/schemas/Community" } }, "total": { "type": "integer" } } },
                "CreateCommunityRequest": { "type": "object", "required": ["name", "category", "visibility"],
                    "properties": { "name": { "type": "string", "minLength": 1, "maxLength": 100 },
                        "description": { "type": "string", "nullable": true, "maxLength": 1000 },
                        "category": { "type": "string", "enum": ["TECH", "BUSINESS", "HOBBY"] },
                        "visibility": { "type": "string", "enum": ["PUBLIC", "PRIVATE"] } } },
                "MemberResponse": { "type": "object", "required": ["id", "communityId", "accountId", "role", "status", "createdAt"],
                    "properties": { "id": { "type": "string", "format": "uuid" }, "communityId": { "type": "string", "format": "uuid" },
                        "accountId": { "type": "string", "format": "uuid" },
                        "role": { "type": "string", "enum": ["OWNER", "ADMIN", "MEMBER"] },
                        "status": { "type": "string", "enum": ["PENDING", "ACTIVE"] },
                        "createdAt": { "type": "string", "format": "date-time" } } },
                "MemberRead": { "type": "object", "required": ["id", "communityId", "accountName", "role", "status", "createdAt"],
                    "properties": { "id": { "type": "string", "format": "uuid" }, "communityId": { "type": "string", "format": "uuid" },
                        "accountId": { "type": "string", "format": "uuid", "description": "閲覧者が当該コミュニティの ACTIVE メンバーの場合のみ返る" },
                        "accountName": { "type": "string" },
                        "role": { "type": "string", "enum": ["OWNER", "ADMIN", "MEMBER"] },
                        "status": { "type": "string", "enum": ["PENDING", "ACTIVE"] },
                        "createdAt": { "type": "string", "format": "date-time" } } },
                "MemberListResponse": { "type": "object",
                    "properties": { "members": { "type": "array", "items": { "$ref": "#/components/schemas/MemberRead" } }, "total": { "type": "integer" } } },
                "Event": { "type": "object",
                    "properties": { "id": { "type": "string", "format": "uuid" }, "communityId": { "type": "string", "format": "uuid" },
                        "title": { "type": "string" }, "description": { "type": "string", "nullable": true },
                        "startsAt": { "type": "string", "format": "date-time" }, "endsAt": { "type": "string", "format": "date-time" },
                        "format": { "type": "string", "enum": ["ONLINE", "OFFLINE", "HYBRID"] },
                        "capacity": { "type": "integer", "minimum": 1, "maximum": 1000 },
                        "status": { "type": "string", "enum": ["DRAFT", "PUBLISHED", "CLOSED", "CANCELLED"] },
                        "createdAt": { "type": "string", "format": "date-time" }, "updatedAt": { "type": "string", "format": "date-time" } } },
                "EventResponse": { "type": "object", "properties": { "event": { "$ref": "#/components/schemas/Event" } } },
                "ListCommunityEventsResponse": { "type": "object",
                    "properties": { "events": { "type": "array", "items": { "$ref": "#/components/schemas/Event" } }, "total": { "type": "integer" } } },
                "CreateEventRequest": { "type": "object", "required": ["title", "startsAt", "endsAt", "format", "capacity"],
                    "properties": { "title": { "type": "string", "minLength": 1, "maxLength": 100 },
                        "description": { "type": "string", "nullable": true, "maxLength": 1000 },
                        "startsAt": { "type": "string", "format": "date-time" }, "endsAt": { "type": "string", "format": "date-time" },
                        "format": { "type": "string", "enum": ["ONLINE", "OFFLINE", "HYBRID"] },
                        "capacity": { "type": "integer", "minimum": 1, "maximum": 1000 } } }
            }
        },
        "paths": {
            "/health": { "get": { "tags": ["System"], "summary": "Health check",
                "responses": { "200": { "description": "Application is running" } } } },
            "/auth/register": { "post": { "tags": ["Auth"], "summary": "アカウント登録",
                "requestBody": json_body("RegisterRequest"),
                "responses": { "201": ok_json("登録成功", "AuthResponse"), "400": resp("バリデーションエラー"), "409": resp("メールアドレス重複") } } },
            "/auth/login": { "post": { "tags": ["Auth"], "summary": "ログイン",
                "requestBody": json_body("LoginRequest"),
                "responses": { "200": ok_json("ログイン成功", "AuthResponse"), "400": resp("バリデーションエラー"), "401": resp("認証失敗") } } },
            "/auth/me": { "get": { "tags": ["Auth"], "summary": "現在のアカウント情報", "security": bearer,
                "responses": { "200": { "description": "取得成功" }, "401": resp("認証エラー"), "404": resp("アカウントなし") } } },
            "/communities": {
                "post": { "tags": ["Communities"], "summary": "コミュニティを作成する", "security": bearer,
                    "requestBody": json_body("CreateCommunityRequest"),
                    "responses": { "201": ok_json("作成成功", "CommunityResponse"), "400": resp("バリデーションエラー"),
                        "401": resp("認証エラー"), "409": resp("コミュニティ名重複"), "422": resp("コミュニティ数上限") } },
                "get": { "tags": ["Communities"], "summary": "コミュニティ一覧を取得する",
                    "description": "?member=me で自分が所属するコミュニティのみ取得（認証必要）。",
                    "parameters": list_communities_params,
                    "responses": { "200": ok_json("取得成功", "ListCommunitiesResponse"), "401": resp("認証エラー") } }
            },
            "/communities/{id}": { "get": { "tags": ["Communities"], "summary": "コミュニティを取得する",
                "parameters": [id_param("id")],
                "responses": { "200": ok_json("取得成功", "CommunityResponse"), "404": resp("見つからない") } } },
            "/communities/{id}/members": {
                "post": { "tags": ["Members"], "summary": "コミュニティに参加する", "security": bearer, "parameters": [id_param("id")],
                    "responses": { "201": ok_json("参加成功", "MemberResponse"), "401": resp("認証エラー"), "404": resp("見つからない"), "409": resp("参加済み") } },
                "get": { "tags": ["Members"], "summary": "メンバー一覧を取得する",
                    "description": "既定は ACTIVE メンバーのみ。閲覧者が ACTIVE な OWNER / ADMIN のときは承認待ち（PENDING）も先頭に含め、total もそれを数える。",
                    "parameters": [id_param("id"), page_params()[0].clone(), page_params()[1].clone()],
                    "responses": { "200": ok_json("取得成功", "MemberListResponse"), "404": resp("見つからない") } }
            },
            "/communities/{id}/members/me": { "delete": { "tags": ["Members"], "summary": "コミュニティを脱退する", "security": bearer,
                "parameters": [id_param("id")],
                "responses": { "200": { "description": "脱退成功" }, "401": resp("認証エラー"), "404": resp("見つからない"), "422": resp("オーナーは脱退不可") } } },
            "/communities/{id}/members/{memberId}": { "delete": { "tags": ["Members"], "summary": "メンバーID指定で脱退する", "security": bearer,
                "parameters": [id_param("id"), id_param("memberId")],
                "responses": { "200": { "description": "脱退成功" }, "401": resp("認証エラー"), "404": resp("見つからない"), "422": resp("オーナーは脱退不可") } } },
            "/communities/{id}/members/{memberId}/approve": { "patch": { "tags": ["Members"], "summary": "メンバーを承認する", "security": bearer,
                "parameters": [id_param("id"), id_param("memberId")],
                "responses": { "200": ok_json("承認成功", "MemberResponse"), "401": resp("認証エラー"), "403": resp("権限なし"),
                    "404": resp("見つからない"), "422": resp("既にアクティブ") } } },
            "/communities/{id}/members/{memberId}/reject": { "patch": { "tags": ["Members"], "summary": "メンバーを拒否する", "security": bearer,
                "parameters": [id_param("id"), id_param("memberId")],
                "responses": { "204": { "description": "拒否成功" }, "401": resp("認証エラー"), "403": resp("権限なし"), "404": resp("見つからない") } } },
            "/communities/{id}/events": {
                "post": { "tags": ["Events"], "summary": "イベントを作成する（DRAFT）", "security": bearer, "parameters": [id_param("id")],
                    "requestBody": json_body("CreateEventRequest"),
                    "responses": { "201": ok_json("作成成功", "EventResponse"), "400": resp("バリデーションエラー"), "401": resp("認証エラー"),
                        "403": resp("権限なし"), "404": resp("見つからない"), "422": resp("日時不正") } },
                "get": { "tags": ["Events"], "summary": "コミュニティのイベント一覧を取得する",
                    "parameters": [id_param("id"), page_params()[0].clone(), page_params()[1].clone()],
                    "responses": { "200": ok_json("取得成功", "ListCommunityEventsResponse") } }
            }
        }
    })
}

pub const SWAGGER_UI_HTML: &str = r##"<!doctype html>
<html lang="ja">
  <head>
    <meta charset="utf-8" />
    <title>Meetup API Docs</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5.17.14/swagger-ui.css"
      integrity="sha384-wxLW6kwyHktdDGr6Pv1zgm/VGJh99lfUbzSn6HNHBENZlCN7W602k9VkGdxuFvPn" crossorigin="anonymous" />
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5.17.14/swagger-ui-bundle.js"
      integrity="sha384-wmyclcVGX/WhUkdkATwhaK1X1JtiNrr2EoYJ+diV3vj4v6OC5yCeSu+yW13SYJep" crossorigin="anonymous"></script>
    <script>
      window.ui = SwaggerUIBundle({ url: "/openapi.json", dom_id: "#swagger-ui" });
    </script>
  </body>
</html>
"##;
