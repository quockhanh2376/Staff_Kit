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

      /* â”€â”€ Mode toggle â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */
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

      /* â”€â”€ Form â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */
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
        <input id="asset-search" autocomplete="off" placeholder="ASSET-001 or Dell Latitude" />
        <div class="helper" id="search-helper">Only in-stock assets are searchable.</div>

        <div id="selected-assets"></div>
        <div id="asset-results" class="asset-list"></div>
        <button id="submit-button" type="button">Submit Borrow Request</button>
        <div id="message"></div>
      </div>
    </main>

    <script>
      // â”€â”€ State â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
      let mode = "borrow"; // "borrow" | "return"
      const selectedAssets = new Map();

      // â”€â”€ Element refs â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
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

      // â”€â”€ Mode switch â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
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
        resultsEl.innerHTML = "";
        setMessage("");
      }

      btnBorrow.addEventListener("click", () => applyMode("borrow"));
      btnReturn.addEventListener("click", () => applyMode("return"));

      // â”€â”€ Render helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
      const renderSelected = () => {
        if (selectedAssets.size === 0) { selectedEl.innerHTML = ""; return; }
        const chips = Array.from(selectedAssets.values())
          .map((asset) => `
            <span class="selected-chip">
              ${asset.assetCode}
              <button type="button" data-remove="${asset.assetCode}">Ã—</button>
            </span>
          `)
          .join("");
        selectedEl.innerHTML = `<div class="helper">Selected Assets</div><div>${chips}</div>`;
        selectedEl.querySelectorAll("[data-remove]").forEach((btn) => {
          btn.addEventListener("click", () => {
            selectedAssets.delete(btn.getAttribute("data-remove"));
            renderSelected();
          });
        });
      };

      const setMessage = (text, isSuccess = false) => {
        if (!text) { messageEl.innerHTML = ""; return; }
        messageEl.innerHTML = `<div class="message ${isSuccess ? "success" : ""}">${text}</div>`;
      };

      const renderResults = (items) => {
        if (!Array.isArray(items) || items.length === 0) {
          resultsEl.innerHTML = `<div class="helper">No assets matched your search.</div>`;
          return;
        }
        const isReturn = mode === "return";
        resultsEl.innerHTML = items
          .map((asset) => `
            <div class="asset-item">
              <div><strong>${asset.assetCode}</strong></div>
              <div>${asset.assetType} Â· ${asset.displayName}</div>
              <div class="helper">${asset.model ?? ""} ${asset.serialNumber ?? ""}</div>
              <button type="button" class="${isReturn ? "return-mode" : ""}" data-add="${asset.assetCode}">${MODES[mode].addLabel}</button>
            </div>
          `)
          .join("");
        resultsEl.querySelectorAll("[data-add]").forEach((btn) => {
          btn.addEventListener("click", () => {
            const assetCode = btn.getAttribute("data-add");
            const asset = items.find((item) => item.assetCode === assetCode);
            if (!asset) return;
            selectedAssets.set(asset.assetCode, asset);
            renderSelected();
          });
        });
      };

      // â”€â”€ Asset search â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
      let searchTimeout = null;
      searchInput.addEventListener("input", () => {
        clearTimeout(searchTimeout);
        searchTimeout = setTimeout(async () => {
          try {
            const q = encodeURIComponent(searchInput.value.trim());
            const res = await fetch(`${MODES[mode].endpoint}?q=${q}`);
            renderResults(await res.json());
          } catch (err) {
            setMessage(err instanceof Error ? err.message : "Asset search failed.");
          }
        }, 180);
      });

      // â”€â”€ Submit â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
      submitBtn.addEventListener("click", async () => {
        try {
          const submittedEmployeeId = document.getElementById("staff-id").value.trim();
          const submittedFullName   = document.getElementById("full-name").value.trim();
          const assetCodes          = Array.from(selectedAssets.keys());

          const res = await fetch("/api/borrow-requests", {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({
              submittedEmployeeId,
              submittedFullName,
              assetCodes,
              requestType: mode,
            }),
          });

          const payload = await res.json();
          if (!res.ok) { setMessage(payload.error || "Submit failed."); return; }

          selectedAssets.clear();
          renderSelected();
          const verb = mode === "return" ? "Return" : "Borrow";
          setMessage(`${verb} request ${payload.requestKey} submitted. IT will review it shortly.`, true);
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
}