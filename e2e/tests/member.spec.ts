import {
  test,
  expect,
  type APIRequestContext,
  type Page,
} from "@playwright/test";

const uniqueSuffix = () => Date.now().toString();

async function registerUser(page: Page, suffix: string, prefix = "member") {
  await page.goto("/register");
  await page.getByLabel("名前").fill(`メンバーユーザー${suffix}`);
  await page
    .getByLabel("メールアドレス")
    .fill(`${prefix}${suffix}@example.com`);
  await page.getByLabel("パスワード").fill("password123");
  await page.getByRole("button", { name: "登録" }).click();
  await expect(page).toHaveURL("/");
}

async function createCommunity(page: Page, name: string) {
  await page.getByRole("link", { name: "コミュニティ作成" }).click();
  await expect(
    page.getByRole("heading", { name: "コミュニティ作成" }),
  ).toBeVisible();
  await page.getByLabel("コミュニティ名").fill(name);
  await page.locator("textarea#description").fill(`${name}の説明`);
  await page.getByRole("button", { name: "作成" }).click();
  await expect(page.getByRole("heading", { name, exact: true })).toBeVisible();
}

async function navigateToCommunity(page: Page, name: string) {
  await page.getByRole("link", { name: "コミュニティ一覧" }).click();
  await page.getByRole("heading", { name, level: 2, exact: true }).click();
  await expect(
    page.getByRole("heading", { name, level: 1, exact: true }),
  ).toBeVisible();
}

async function logout(page: Page) {
  await page.getByRole("button", { name: "ログアウト" }).click();
  await expect(page.getByRole("link", { name: "ログイン" })).toBeVisible();
}

async function login(page: Page, email: string, password = "password123") {
  await page.goto("/login");
  await page.getByLabel("メールアドレス").fill(email);
  await page.getByLabel("パスワード").fill(password);
  await page.getByRole("button", { name: "ログイン" }).click();
  await expect(page).toHaveURL("/");
}

/** 非公開コミュニティを作成し、遷移先 URL からコミュニティ ID を返す */
async function createPrivateCommunity(page: Page, name: string) {
  await page.getByRole("link", { name: "コミュニティ作成" }).click();
  await page.getByLabel("コミュニティ名").fill(name);
  await page.locator("textarea#description").fill(`${name}の説明`);
  await page.locator("select#visibility").selectOption("PRIVATE");
  await page.getByRole("button", { name: "作成" }).click();
  await expect(page.getByRole("heading", { name, exact: true })).toBeVisible();
  return new URL(page.url()).pathname.split("/").pop() ?? "";
}

/**
 * 申請者を登録して参加申請する。非公開コミュニティは非メンバーに 404 で
 * UI からは辿れないため、API を直接呼ぶ
 */
async function applyViaApi(
  request: APIRequestContext,
  communityId: string,
  name: string,
  email: string,
) {
  const registered = await request.post("/auth/register", {
    data: { name, email, password: "password123" },
  });
  expect(registered.status()).toBe(201);
  const { token } = await registered.json();
  const applied = await request.post(`/communities/${communityId}/members`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(applied.status()).toBe(201);
  expect((await applied.json()).status).toBe("PENDING");
}

/** マイコミュニティ経由で詳細ページを開き直し、メンバー一覧を再取得させる */
async function reopenFromMyCommunities(page: Page, name: string) {
  await page.getByRole("link", { name: "マイコミュニティ" }).click();
  await page.getByRole("heading", { name, level: 2, exact: true }).click();
  await expect(
    page.getByRole("heading", { name, level: 1, exact: true }),
  ).toBeVisible();
}

/** メンバー一覧のうち、指定した名前のメンバーのカード */
function memberCard(page: Page, name: string) {
  return page
    .locator("div.rounded-lg")
    .filter({ has: page.getByText(name, { exact: true }) });
}

test.describe("メンバー", () => {
  test.describe("参加申請の審査", () => {
    test("オーナーは非公開コミュニティの参加申請を承認できる", async ({
      page,
    }) => {
      const suffix = uniqueSuffix();
      const communityName = `承認テストコミュニティ${suffix}`;
      const applicantName = `承認申請者${suffix}`;
      await registerUser(page, suffix, "approve-owner");
      const communityId = await createPrivateCommunity(page, communityName);
      await applyViaApi(
        page.request,
        communityId,
        applicantName,
        `approve-applicant${suffix}@example.com`,
      );

      await reopenFromMyCommunities(page, communityName);
      const card = memberCard(page, applicantName);
      await expect(
        card.getByText("MEMBER / PENDING", { exact: true }),
      ).toBeVisible();

      await card.getByRole("button", { name: "承認", exact: true }).click();

      await expect(
        card.getByText("MEMBER / ACTIVE", { exact: true }),
      ).toBeVisible();
      await expect(card.getByRole("button")).toHaveCount(0);
    });

    test("オーナーは非公開コミュニティの参加申請を拒否できる", async ({
      page,
    }) => {
      const suffix = uniqueSuffix();
      const communityName = `拒否テストコミュニティ${suffix}`;
      const applicantName = `拒否申請者${suffix}`;
      await registerUser(page, suffix, "reject-owner");
      const communityId = await createPrivateCommunity(page, communityName);
      await applyViaApi(
        page.request,
        communityId,
        applicantName,
        `reject-applicant${suffix}@example.com`,
      );

      await reopenFromMyCommunities(page, communityName);
      const card = memberCard(page, applicantName);
      await expect(
        card.getByText("MEMBER / PENDING", { exact: true }),
      ).toBeVisible();

      await card.getByRole("button", { name: "拒否", exact: true }).click();

      await expect(card).toHaveCount(0);
    });
  });

  test.describe("コミュニティ参加", () => {
    test("コミュニティに参加するとメンバーとして表示される", async ({
      page,
    }) => {
      const suffix = uniqueSuffix();
      const communityName = `参加テストコミュニティ${suffix}`;
      const ownerEmail = `owner${suffix}@example.com`;
      const memberEmail = `joiner${suffix}@example.com`;

      // オーナーとして登録・コミュニティ作成
      await page.goto("/register");
      await page.getByLabel("名前").fill(`オーナー${suffix}`);
      await page.getByLabel("メールアドレス").fill(ownerEmail);
      await page.getByLabel("パスワード").fill("password123");
      await page.getByRole("button", { name: "登録" }).click();
      await expect(page).toHaveURL("/");

      await createCommunity(page, communityName);

      // ログアウト
      await logout(page);

      // 別ユーザーとして登録
      await page.goto("/register");
      await page.getByLabel("名前").fill(`参加者${suffix}`);
      await page.getByLabel("メールアドレス").fill(memberEmail);
      await page.getByLabel("パスワード").fill("password123");
      await page.getByRole("button", { name: "登録" }).click();
      await expect(page).toHaveURL("/");

      // コミュニティ詳細ページへ（クライアントサイドナビゲーション）
      await navigateToCommunity(page, communityName);

      // 参加ボタンが表示されている
      await expect(
        page.getByRole("button", { name: "参加する" }),
      ).toBeVisible();

      // 参加する
      await page.getByRole("button", { name: "参加する" }).click();

      // 退会ボタンが表示される（参加済み）
      await expect(
        page.getByRole("button", { name: "退会する" }),
      ).toBeVisible();
    });
  });

  test.describe("コミュニティ退会", () => {
    test("コミュニティから退会できる", async ({ page }) => {
      const suffix = uniqueSuffix();
      const communityName = `退会テストコミュニティ${suffix}`;
      const ownerEmail = `owner2${suffix}@example.com`;
      const memberEmail = `leaver${suffix}@example.com`;

      // オーナーとして登録・コミュニティ作成
      await page.goto("/register");
      await page.getByLabel("名前").fill(`オーナー2${suffix}`);
      await page.getByLabel("メールアドレス").fill(ownerEmail);
      await page.getByLabel("パスワード").fill("password123");
      await page.getByRole("button", { name: "登録" }).click();
      await expect(page).toHaveURL("/");

      await createCommunity(page, communityName);

      await logout(page);

      // メンバーとして登録・参加
      await page.goto("/register");
      await page.getByLabel("名前").fill(`退会者${suffix}`);
      await page.getByLabel("メールアドレス").fill(memberEmail);
      await page.getByLabel("パスワード").fill("password123");
      await page.getByRole("button", { name: "登録" }).click();
      await expect(page).toHaveURL("/");

      // コミュニティ詳細ページへ（クライアントサイドナビゲーション）
      await navigateToCommunity(page, communityName);
      await page.getByRole("button", { name: "参加する" }).click();
      await expect(
        page.getByRole("button", { name: "退会する" }),
      ).toBeVisible();

      // 退会する
      await page.getByRole("button", { name: "退会する" }).click();

      // 参加ボタンが再表示される
      await expect(
        page.getByRole("button", { name: "参加する" }),
      ).toBeVisible();
    });
  });
});
