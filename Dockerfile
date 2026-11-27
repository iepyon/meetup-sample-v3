FROM node:26-bookworm

# Node は Claude Code CLI 用のみ。アプリ本体・E2E は Rust (axum + Leptos / chromiumoxide)。
# System dependencies: git, SQLite tools, jq, build tools, E2E 用の Chromium
RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    sqlite3 \
    jq \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    # E2E（e2e/ の chromiumoxide）が CDP で操作するヘッドレスブラウザ。Debian 版は arm64 / amd64 両対応
    chromium \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user matching macOS UID for bind mount permissions
ARG USER_UID=501
ARG USER_GID=20
RUN groupadd --gid $USER_GID devuser 2>/dev/null || true \
    && useradd --uid $USER_UID --gid $USER_GID -m devuser \
    && mkdir -p /repo && chown devuser:$USER_GID /repo

# Install gosu for entrypoint user switching
RUN apt-get update && apt-get install -y --no-install-recommends gosu && rm -rf /var/lib/apt/lists/*

# Rust toolchain（rustup は root で共有パスに導入し、devuser から使えるようにする）
# wasm32 ターゲットは Leptos (CSR) のビルドに、clippy / rustfmt は品質ゲートに必要
ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:$PATH
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable \
        --component clippy --component rustfmt --target wasm32-unknown-unknown \
    && cargo install trunk --locked --version 0.22.2 \
    && chown -R devuser:$USER_GID /opt/rustup /opt/cargo

# wasm-bindgen-cli をソースからビルドして入れる。trunk が自動ダウンロードする aarch64 版の
# 配布バイナリは GLIBC 2.38 を要求し、bookworm（2.36）では動かないため（PATH に同じ版があれば trunk はそれを使う）。
# バージョンは Cargo.lock の wasm-bindgen と必ず揃えること（ずれると trunk が再びダウンロードを試みて失敗する）
# ビルド用のレジストリは一時 CARGO_HOME に置いて捨て、/opt/cargo 全体を chown し直さない（レイヤ肥大の回避）
RUN CARGO_HOME=/tmp/cargo-build cargo install wasm-bindgen-cli --locked --version 0.2.130 --root /opt/cargo \
    && rm -rf /tmp/cargo-build \
    && chown devuser:$USER_GID /opt/cargo/bin/wasm-bindgen* /opt/cargo/.crates.toml /opt/cargo/.crates2.json

# Allow git to work with bind-mounted repos (different owner)
RUN git config --system --add safe.directory '*'

# Claude Code CLI (root でインストール、devuser から実行可能)
RUN npm install -g @anthropic-ai/claude-code

# lefthook for git hooks (アーキテクチャ自動判定: arm64 ネイティブ実行に対応)
# ダウンロードした .gz を公式 sha256 で検証してから展開（サプライチェーン対策）
RUN set -e; \
    case "$(dpkg --print-architecture)" in \
      arm64) LH_ARCH=arm64; LH_SHA=417d57ef4b67863b34f7f8b7f52bf9f7ab4cd0ffc2047d49f4efb15e1e210b58 ;; \
      amd64) LH_ARCH=x86_64; LH_SHA=fd164d97440108fe032377a1ff62228d7cff46e709c8475758b02313dbaf6896 ;; \
      *) echo "unsupported arch: $(dpkg --print-architecture)" >&2; exit 1 ;; \
    esac; \
    curl -fsSL "https://github.com/evilmartians/lefthook/releases/download/v1.10.4/lefthook_1.10.4_Linux_${LH_ARCH}.gz" -o lefthook.gz \
    && echo "${LH_SHA}  lefthook.gz" | sha256sum -c - \
    && gunzip lefthook.gz \
    && chmod +x lefthook \
    && mv lefthook /usr/local/bin/lefthook

COPY scripts/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

WORKDIR /workspace
ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["sleep", "infinity"]
