pub(crate) fn borrow_page_html() -> &'static str {
    r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Staff Kit</title>
    <style>
      :root {
        color-scheme: dark;
        --bg: #0e1117;
        --surface: #151922;
        --surface-2: #1c2230;
        --border: rgba(255, 255, 255, 0.12);
        --text: #f3f4f6;
        --muted: #a7b0c0;
        --accent: #10b981;
        --accent-text: #052e16;
        --accent-hover: #059669;
        --success: #45d483;
      }

      * { box-sizing: border-box; }

      body {
        margin: 0;
        min-height: 100vh;
        font-family: "Segoe UI", sans-serif;
        background:
          radial-gradient(circle at top, rgba(255, 216, 77, 0.16), transparent 34%),
          var(--bg);
        color: var(--text);
      }

      main {
        width: min(100%, 720px);
        margin: 0 auto;
        padding: 24px 16px 40px;
      }

      .card {
        border: 1px solid var(--border);
        background: linear-gradient(180deg, rgba(255, 255, 255, 0.03), transparent), var(--surface);
        border-radius: 18px;
        padding: 18px;
        box-shadow: 0 24px 80px rgba(0, 0, 0, 0.28);
      }

      h1 {
        margin: 0;
        font-size: 28px;
      }

      p {
        color: var(--muted);
        line-height: 1.5;
      }

      /* Mode toggle */
      .mode-toggle {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 8px;
        margin-top: 16px;
        margin-bottom: 4px;
        background: var(--surface-2);
        border: 1px solid var(--border);
        border-radius: 14px;
        padding: 6px;
      }

      .mode-btn {
        width: 100%;
        margin: 0;
        padding: 10px 8px;
        border-radius: 10px;
        border: none;
        background: transparent;
        color: var(--muted);
        font-size: 14px;
        font-weight: 600;
        cursor: pointer;
        transition: background 140ms ease, color 140ms ease;
      }

      .mode-btn.active-borrow {
        background: var(--accent);
        color: var(--accent-text);
      }

      .mode-btn.active-return {
        background: #f59e0b;
        color: #1c0a00;
      }

      /* Form */
      label {
        display: block;
        margin-top: 14px;
        margin-bottom: 6px;
        font-size: 13px;
        font-weight: 600;
      }

      input, button {
        width: 100%;
        border-radius: 12px;
        border: 1px solid var(--border);
        padding: 12px 14px;
        font: inherit;
      }

      .search-row {
        display: flex;
        gap: 8px;
        align-items: stretch;
      }

      .search-row input {
        flex: 1;
        width: auto;
      }

      #search-btn {
        width: auto;
        flex-shrink: 0;
        padding: 12px 18px;
        background: var(--surface-2);
        color: var(--text);
        font-weight: 600;
        border: 1px solid var(--border);
        cursor: pointer;
        transition: background 140ms ease;
      }

      #search-btn:hover {
        background: var(--surface);
      }

      input {
        background: var(--surface-2);
        color: var(--text);
      }

      #submit-button {
        margin-top: 18px;
        background: var(--accent);
        color: var(--accent-text);
        font-weight: 700;
        transition: background-color 160ms ease;
      }

      #submit-button:hover {
        background: var(--accent-hover);
      }

      #submit-button.return-mode {
        background: #f59e0b;
        color: #1c0a00;
      }

      #submit-button.return-mode:hover {
        background: #d97706;
      }

      .helper {
        margin-top: 10px;
        font-size: 12px;
        color: var(--muted);
      }

      .asset-list {
        margin-top: 14px;
        display: grid;
        gap: 10px;
      }

      .asset-item {
        border: 1px solid var(--border);
        border-radius: 12px;
        background: var(--surface-2);
        padding: 12px;
      }

      .asset-item button {
        margin-top: 10px;
        background: var(--accent);
        color: var(--accent-text);
        font-weight: 600;
      }

      .asset-item button.return-mode {
        background: #f59e0b;
        color: #1c0a00;
      }

      .selected-chip {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        margin: 6px 6px 0 0;
        padding: 8px 10px;
        border-radius: 999px;
        background: rgba(255, 216, 77, 0.16);
        color: var(--text);
      }

      .selected-chip button {
        width: auto;
        margin: 0;
        padding: 0;
        border: 0;
        background: transparent;
        color: var(--text);
      }

      .message {
        margin-top: 14px;
        border-radius: 12px;
        padding: 12px 14px;
        background: rgba(255, 255, 255, 0.04);
        color: var(--text);
      }

      .message.success {
        border: 1px solid rgba(69, 212, 131, 0.4);
        background: rgba(69, 212, 131, 0.12);
      }
    </style>
  </head>
  <body>
    <main>
      <div class="card">
        <h1 id="page-title">Borrow Asset</h1>
        <p id="page-desc">Enter your Staff ID, full name, then search and select the asset items you are receiving from IT.</p>

        <div class="mode-toggle">
          <button class="mode-btn active-borrow" id="btn-borrow" type="button">&#x2193; Borrow</button>
          <button class="mode-btn" id="btn-return" type="button">&#x2191; Return</button>
        </div>

        <label for="staff-id">Staff ID</label>
        <input id="staff-id" autocomplete="off" placeholder="EE1001" />

        <label for="full-name">Full Name</label>
        <input id="full-name" autocomplete="name" placeholder="Nguyen Van A" />

        <label for="asset-search">Search Asset</label>
        <div class="search-row">
          <input id="asset-search" autocomplete="off" placeholder="ASSET-001 or Dell Latitude" />
          <button id="search-btn" type="button">Search</button>
        </div>
        <div class="helper" id="search-helper">Only in-stock assets are searchable.</div>

        <div id="selected-assets"></div>
        <div id="asset-results" class="asset-list"></div>
        <button id="submit-button" type="button">Submit Borrow Request</button>
        <div id="message"></div>
      </div>
    </main>

    <script>
      // State
      let mode = "borrow"; // "borrow" | "return"
      const selectedAssets = new Map();

      // Element refs
      const resultsEl   = document.getElementById("asset-results");
      const selectedEl  = document.getElementById("selected-assets");
      const messageEl   = document.getElementById("message");
      const searchInput = document.getElementById("asset-search");
      const submitBtn   = document.getElementById("submit-button");
      const pageTitle   = document.getElementById("page-title");
      const pageDesc    = document.getElementById("page-desc");
      const searchHelper = document.getElementById("search-helper");
      const btnBorrow   = document.getElementById("btn-borrow");
      const btnReturn   = document.getElementById("btn-return");
      const searchBtn   = document.getElementById("search-btn");

      // The QR token is carried in the fragment so it is never sent to the
      // LAN server in the page request, access logs, or referrer headers.
      const fragmentParams = new URLSearchParams(location.hash.slice(1));
      const lanToken = fragmentParams.get("t") || "";
      if (location.hash) {
        history.replaceState(null, "", location.pathname + location.search);
      }
      const clientSessionId = crypto.randomUUID();

      const authorizedFetch = (input, init = {}) => {
        if (!lanToken) {
          throw new Error("LAN access token is missing. Scan the current QR code again.");
        }
        const headers = new Headers(init.headers || {});
        headers.set("Authorization", `Bearer ${lanToken}`);
        return fetch(input, { ...init, headers });
      };

      // Mode switch
      const MODES = {
        borrow: {
          title: "Borrow Asset",
          desc: "Enter your Staff ID, full name, then search and select the asset items you are receiving from IT.",
          helper: "Only in-stock assets are searchable.",
          submit: "Submit Borrow Request",
          endpoint: "/api/assets",
          addLabel: "Add Asset",
        },
        return: {
          title: "Return Asset",
          desc: "Enter your Staff ID, full name, then search and select the assigned asset items you are returning to IT.",
          helper: "Only currently assigned (borrowed) assets are searchable.",
          submit: "Submit Return Request",
          endpoint: "/api/assigned-assets",
          addLabel: "Return this Asset",
        },
      };

      function applyMode(newMode) {
        mode = newMode;
        const cfg = MODES[mode];

        pageTitle.textContent = cfg.title;
        pageDesc.textContent = cfg.desc;
        searchHelper.textContent = cfg.helper;
        submitBtn.textContent = cfg.submit;

        if (mode === "return") {
          submitBtn.classList.add("return-mode");
          btnReturn.classList.add("active-return");
          btnReturn.classList.remove("active-borrow");
          btnBorrow.classList.remove("active-borrow", "active-return");
        } else {
          submitBtn.classList.remove("return-mode");
          btnBorrow.classList.add("active-borrow");
          btnBorrow.classList.remove("active-return");
          btnReturn.classList.remove("active-borrow", "active-return");
        }

        // Clear state when switching mode
        selectedAssets.clear();
        renderSelected();
        resultsEl.replaceChildren();
        setMessage("");
      }

      btnBorrow.addEventListener("click", () => applyMode("borrow"));
      btnReturn.addEventListener("click", () => applyMode("return"));

      // Render helpers
      const renderSelected = () => {
        selectedEl.replaceChildren();
        if (selectedAssets.size === 0) return;
        const heading = document.createElement("div");
        heading.className = "helper";
        heading.textContent = "Selected Assets";
        const chips = document.createElement("div");
        for (const asset of selectedAssets.values()) {
          const chip = document.createElement("span");
          chip.className = "selected-chip";
          chip.textContent = asset.assetCode;
          const remove = document.createElement("button");
          remove.type = "button";
          remove.dataset.remove = asset.assetCode;
          remove.textContent = "×";
          remove.addEventListener("click", () => {
            selectedAssets.delete(asset.assetCode);
            renderSelected();
          });
          chip.append(remove);
          chips.append(chip);
        }
        selectedEl.append(heading, chips);
      };

      const setMessage = (text, isSuccess = false) => {
        messageEl.replaceChildren();
        if (!text) return;
        const message = document.createElement("div");
        message.className = isSuccess ? "message success" : "message";
        message.textContent = text;
        messageEl.append(message);
      };

      const renderResults = (items) => {
        resultsEl.replaceChildren();
        if (!Array.isArray(items) || items.length === 0) {
          const empty = document.createElement("div");
          empty.className = "helper";
          empty.textContent = "No assets matched your search.";
          resultsEl.append(empty);
          return;
        }
        const isReturn = mode === "return";
        for (const asset of items) {
          const item = document.createElement("div");
          item.className = "asset-item";
          const code = document.createElement("div");
          const strong = document.createElement("strong");
          strong.textContent = asset.assetCode;
          code.append(strong);
          const description = document.createElement("div");
          description.textContent = `${asset.assetType} - ${asset.displayName}`;
          const metadata = document.createElement("div");
          metadata.className = "helper";
          metadata.textContent = `${asset.model ?? ""} ${asset.serialNumber ?? ""}`;
          const add = document.createElement("button");
          add.type = "button";
          add.className = isReturn ? "return-mode" : "";
          add.dataset.add = asset.assetCode;
          add.textContent = MODES[mode].addLabel;
          add.addEventListener("click", () => {
            selectedAssets.set(asset.assetCode, asset);
            renderSelected();
          });
          item.append(code, description, metadata, add);
          resultsEl.append(item);
        }
      };

      // -- Asset search --------------------------------------------------------
      const doSearch = async () => {
        try {
          const q = encodeURIComponent(searchInput.value.trim());
          const res = await authorizedFetch(`${MODES[mode].endpoint}?q=${q}`);
          renderResults(await res.json());
        } catch (err) {
          setMessage(err instanceof Error ? err.message : "Asset search failed.");
        }
      };

      searchBtn.addEventListener("click", doSearch);
      searchInput.addEventListener("keydown", (e) => { if (e.key === "Enter") doSearch(); });

      // Submit
      submitBtn.addEventListener("click", async () => {
        try {
          const submittedEmployeeId = document.getElementById("staff-id").value.trim();
          const submittedFullName   = document.getElementById("full-name").value.trim();
          const assetCodes          = Array.from(selectedAssets.keys());

          const res = await authorizedFetch("/api/borrow-requests", {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({
              submittedEmployeeId,
              submittedFullName,
              assetCodes,
              requestType: mode,
              requestId: crypto.randomUUID(),
              clientSessionId,
            }),
          });

          const payload = await res.json();
          if (!res.ok) { setMessage(payload.error || "Submit failed."); return; }

          selectedAssets.clear();
          renderSelected();
          const verb = mode === "return" ? "Return" : "Borrow";
          setMessage(`${verb} request ${payload.requestReference} submitted. ${payload.message}`, true);
        } catch (err) {
          setMessage(err instanceof Error ? err.message : "Submit failed.");
        }
      });
    </script>
  </body>
</html>"#
}

#[cfg(test)]
mod tests {
    use super::borrow_page_html;
    use std::io::Write;
    use std::process::{Command, Stdio};

    #[test]
    fn borrow_page_uses_green_submit_button_theme() {
        let html = borrow_page_html();
        assert!(
            html.contains("--accent: #10b981;"),
            "expected borrow page accent to use the app green action color"
        );
        assert!(
            html.contains("--accent-hover: #059669;"),
            "expected borrow page to define --accent-hover CSS variable"
        );
        assert!(
            html.contains("btn-borrow"),
            "expected mode toggle borrow button"
        );
        assert!(
            html.contains("btn-return"),
            "expected mode toggle return button"
        );
    }

    #[test]
    fn borrow_page_does_not_contain_mojibake_sequences() {
        let html = borrow_page_html();

        assert!(
            !html.contains("Â·"),
            "expected borrow page HTML to avoid mojibake middle-dot sequences"
        );
        assert!(
            !html.contains("Ã—"),
            "expected borrow page HTML to avoid mojibake multiplication glyphs"
        );
        assert!(
            !html.contains("â"),
            "expected borrow page HTML comments and labels to avoid mojibake sequences"
        );
    }

    #[test]
    fn borrow_page_uses_fragment_token_and_authorized_fetches() {
        let html = borrow_page_html();

        assert!(html.contains("location.hash"));
        assert!(html.contains("history.replaceState"));
        assert!(html.contains("fragmentParams.get(\"t\")"));
        assert!(html.contains("Authorization"));
        assert!(html.contains("Bearer ${lanToken}"));
        assert!(!html.contains("?t="));
        assert!(!html.contains("fetch(`${MODES[mode].endpoint}?q=${q}`)"));
    }

    #[test]
    fn borrow_page_generates_session_and_submission_request_ids() {
        let html = borrow_page_html();

        assert!(html.contains("const clientSessionId = crypto.randomUUID()"));
        assert!(html.contains("requestId: crypto.randomUUID()"));
        assert!(html.contains("clientSessionId"));
    }

    #[test]
    fn borrow_page_renders_api_values_with_text_nodes_not_html() {
        let html = borrow_page_html();

        assert!(!html.contains("innerHTML"));
        assert!(html.contains("replaceChildren"));

        let script = r###"
import { JSDOM } from "jsdom";
import fs from "node:fs";

const page = fs.readFileSync(0, "utf8");
const payloads = {
  assetCode: '<img src=x onerror="window.__xss=1">',
  assetType: '<script>window.__xss=1</script>',
  displayName: '</div><svg onload="window.__xss=1">',
  model: '<script>window.__xss=1</script>',
  serialNumber: '<img src=x onerror="window.__xss=1">',
  error: '<script>window.__xss=1</script>',
};
const asset = { ...payloads, model: payloads.model, serialNumber: payloads.serialNumber };
const wait = () => new Promise((resolve) => setTimeout(resolve, 20));
const dom = new JSDOM(page, {
  url: "http://127.0.0.1/borrow#t=test-token",
  runScripts: "dangerously",
  beforeParse(window) {
    window.__xss = 0;
    if (!window.crypto.randomUUID) {
      Object.defineProperty(window.crypto, "randomUUID", { value: () => "test-session" });
    }
    window.fetch = async (_input, init = {}) => init.method === "POST"
      ? { ok: false, json: async () => ({ error: payloads.error }) }
      : { ok: true, json: async () => [asset] };
  },
});
const document = dom.window.document;
document.getElementById("search-btn").click();
await wait();
const result = document.querySelector(".asset-item");
if (!result || [payloads.assetCode, payloads.assetType, payloads.displayName, payloads.model, payloads.serialNumber]
  .some((value) => !result.textContent.includes(value))) {
  throw new Error("asset values were not rendered as text");
}
if (result.querySelector("img,script,svg")) throw new Error("asset value became markup");
result.querySelector("[data-add]").click();
const selected = document.querySelector("#selected-assets");
if (!selected.textContent.includes(payloads.assetCode) || selected.querySelector("img,script,svg")) {
  throw new Error("selected asset became markup");
}
document.getElementById("staff-id").value = "EE1001";
document.getElementById("full-name").value = "Test Employee";
document.getElementById("submit-button").click();
await wait();
const message = document.getElementById("message");
if (!message.textContent.includes(payloads.error) || message.querySelector("img,script,svg")) {
  throw new Error("backend error became markup");
}
if (dom.window.__xss !== 0) throw new Error("XSS payload executed");
console.log("safe");
"###;

        let mut child = Command::new("node")
            .args(["--input-type=module", "-e", script])
            .current_dir(std::env::current_dir().expect("workspace directory"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("node and jsdom are required for browser rendering coverage");
        child
            .stdin
            .take()
            .expect("node stdin")
            .write_all(html.as_bytes())
            .expect("write page HTML to browser fixture");
        let output = child.wait_with_output().expect("browser fixture result");
        assert!(
            output.status.success(),
            "browser fixture failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
