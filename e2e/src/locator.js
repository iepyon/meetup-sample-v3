// ページ内で評価するロケーター本体（Playwright の getByRole / getByLabel / getByText 相当の簡易版）。
// (spec, op, arg, token) => state。Rust 側（page.rs）がポーリングしながら繰り返し呼ぶ。
(spec, op, arg, token) => {
  const norm = (s) => (s || "").replace(/\s+/g, " ").trim();
  const matches = (actual, expected, exact) =>
    exact
      ? norm(actual) === norm(expected)
      : norm(actual).toLowerCase().includes(norm(expected).toLowerCase());
  const isVisible = (el) => {
    const r = el.getBoundingClientRect();
    return (
      r.width > 0 &&
      r.height > 0 &&
      getComputedStyle(el).visibility !== "hidden"
    );
  };
  const implicitRole = (el) => {
    const tag = el.tagName.toLowerCase();
    if (/^h[1-6]$/.test(tag)) return "heading";
    if (tag === "button") return "button";
    if (tag === "input" && ["button", "submit", "reset"].includes(el.type))
      return "button";
    if (tag === "a" && el.hasAttribute("href")) return "link";
    return null;
  };
  const roleOf = (el) => el.getAttribute("role") || implicitRole(el);
  const headingLevel = (el) => {
    const m = /^h([1-6])$/i.exec(el.tagName);
    return m ? Number(m[1]) : Number(el.getAttribute("aria-level") || 0);
  };
  const accessibleName = (el) => {
    const aria = el.getAttribute("aria-label");
    if (aria) return aria;
    const by = el.getAttribute("aria-labelledby");
    if (by) {
      return by
        .split(/\s+/)
        .map((id) => document.getElementById(id)?.textContent || "")
        .join(" ");
    }
    if (el.tagName === "INPUT") return el.value;
    return el.textContent;
  };
  const descendants = (roots) => {
    const set = new Set();
    for (const root of roots)
      for (const el of root.querySelectorAll("*")) set.add(el);
    return [...set];
  };
  const SKIP = new Set([
    "SCRIPT",
    "STYLE",
    "NOSCRIPT",
    "TEMPLATE",
    "HEAD",
    "TITLE",
  ]);

  const byRole = (roots, s) =>
    // アクセシビリティツリーに載らない非表示要素は Playwright と同様に除外する
    descendants(roots).filter(
      (el) =>
        roleOf(el) === s.role &&
        isVisible(el) &&
        (s.name == null || matches(accessibleName(el), s.name, s.exact)) &&
        (s.level == null || headingLevel(el) === s.level),
    );
  const byLabel = (roots, s) => {
    const out = new Set();
    for (const el of descendants(roots)) {
      if (
        el.tagName === "LABEL" &&
        el.control &&
        matches(el.textContent, s.text, s.exact)
      ) {
        out.add(el.control);
      }
      const aria = el.getAttribute("aria-label");
      if (aria !== null && matches(aria, s.text, s.exact)) out.add(el);
    }
    return [...out];
  };
  // テキストを含む要素のうち最も内側のもの（子要素が同じテキストに一致しないもの）
  const byText = (roots, s) =>
    descendants(roots).filter(
      (el) =>
        !SKIP.has(el.tagName) &&
        matches(el.textContent, s.text, s.exact) &&
        ![...el.children].some((c) => matches(c.textContent, s.text, s.exact)),
    );
  const byCss = (roots, s) => [
    ...new Set(roots.flatMap((root) => [...root.querySelectorAll(s.selector)])),
  ];

  const resolve = (s, roots) => {
    const scope = s.parent ? resolve(s.parent, roots) : roots;
    const finder = { role: byRole, label: byLabel, text: byText, css: byCss }[
      s.kind
    ];
    const found = finder(scope, s);
    return s.has
      ? found.filter((el) => resolve(s.has, [el]).length > 0)
      : found;
  };

  const els = resolve(spec, [document]);
  const one = els.length === 1 ? els[0] : null;
  const state = {
    count: els.length,
    visible: els.filter(isVisible).length,
    ready: false,
    text: one ? norm(one.textContent) : null,
  };
  if (!one || !isVisible(one) || one.disabled) return state;

  if (op === "mark") {
    // クリック対象に目印を付ける。実際のクリックは CDP のマウスイベントで行う
    one.setAttribute("data-e2e-target", token);
    state.ready = true;
  } else if (op === "fill") {
    // Playwright の fill と同様に値を差し替えて input / change を発火する
    // （datetime-local や select はキー入力では扱えないため）。対象はフォーム部品のみ
    if (!["INPUT", "TEXTAREA", "SELECT"].includes(one.tagName) || one.readOnly) {
      return state;
    }
    one.focus();
    const setter = Object.getOwnPropertyDescriptor(
      Object.getPrototypeOf(one),
      "value",
    ).set;
    setter.call(one, arg);
    one.dispatchEvent(new Event("input", { bubbles: true }));
    one.dispatchEvent(new Event("change", { bubbles: true }));
    // 存在しない option を選んだ場合は値が変わらないので、ready にせず待ち続ける
    state.ready = one.value === arg;
  }
  return state;
};
