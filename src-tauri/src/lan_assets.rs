pub(crate) fn borrow_page_html() -> &'static str {
    r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Staff Kit Borrow</title>
    <style>
      :root {
        color-scheme: dark;
        --bg: #0e1117;
        --surface: #151922;
        --surface-2: #1c2230;
        --border: rgba(255, 255, 255, 0.12);
        --text: #f3f4f6;
        --muted: #a7b0c0;
        --accent: #ffd84d;
        --accent-text: #312600;
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

      button {
        margin-top: 18px;
        background: var(--accent);
        color: var(--accent-text);
        font-weight: 700;
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
        <h1>Borrow Asset</h1>
        <p>Enter your Staff ID, full name, then search and select the asset items you are receiving from IT.</p>

        <label for="staff-id">Staff ID</label>
        <input id="staff-id" autocomplete="off" placeholder="EE1001" />

        <label for="full-name">Full Name</label>
        <input id="full-name" autocomplete="name" placeholder="Nguyen Van A" />

        <label for="asset-search">Search Asset</label>
        <input id="asset-search" autocomplete="off" placeholder="ASSET-001 or Dell Latitude" />
        <div class="helper">Only in-stock assets are searchable.</div>

        <div id="selected-assets"></div>
        <div id="asset-results" class="asset-list"></div>
        <button id="submit-button" type="button">Submit Borrow Request</button>
        <div id="message"></div>
      </div>
    </main>

    <script>
      const selectedAssets = new Map();
      const resultsEl = document.getElementById("asset-results");
      const selectedEl = document.getElementById("selected-assets");
      const messageEl = document.getElementById("message");
      const searchInput = document.getElementById("asset-search");

      const renderSelected = () => {
        if (selectedAssets.size === 0) {
          selectedEl.innerHTML = "";
          return;
        }

        const chips = Array.from(selectedAssets.values())
          .map((asset) => `
            <span class="selected-chip">
              ${asset.assetCode}
              <button type="button" data-remove="${asset.assetCode}">x</button>
            </span>
          `)
          .join("");

        selectedEl.innerHTML = `<div class="helper">Selected Assets</div><div>${chips}</div>`;
        selectedEl.querySelectorAll("[data-remove]").forEach((button) => {
          button.addEventListener("click", () => {
            selectedAssets.delete(button.getAttribute("data-remove"));
            renderSelected();
          });
        });
      };

      const setMessage = (text, isSuccess = false) => {
        if (!text) {
          messageEl.innerHTML = "";
          return;
        }
        messageEl.innerHTML = `<div class="message ${isSuccess ? "success" : ""}">${text}</div>`;
      };

      const renderResults = (items) => {
        if (!Array.isArray(items) || items.length === 0) {
          resultsEl.innerHTML = `<div class="helper">No in-stock assets matched your search.</div>`;
          return;
        }

        resultsEl.innerHTML = items
          .map((asset) => `
            <div class="asset-item">
              <div><strong>${asset.assetCode}</strong></div>
              <div>${asset.assetType} · ${asset.displayName}</div>
              <div class="helper">${asset.model ?? ""} ${asset.serialNumber ?? ""}</div>
              <button type="button" data-add="${asset.assetCode}">Add Asset</button>
            </div>
          `)
          .join("");

        resultsEl.querySelectorAll("[data-add]").forEach((button) => {
          button.addEventListener("click", () => {
            const assetCode = button.getAttribute("data-add");
            const asset = items.find((item) => item.assetCode === assetCode);
            if (!asset) return;
            selectedAssets.set(asset.assetCode, asset);
            renderSelected();
          });
        });
      };

      let searchTimeout = null;
      searchInput.addEventListener("input", () => {
        clearTimeout(searchTimeout);
        searchTimeout = setTimeout(async () => {
          try {
            const query = encodeURIComponent(searchInput.value.trim());
            const response = await fetch(`/api/assets?q=${query}`);
            const payload = await response.json();
            renderResults(payload);
          } catch (error) {
            setMessage(error instanceof Error ? error.message : "Asset search failed.");
          }
        }, 180);
      });

      document.getElementById("submit-button").addEventListener("click", async () => {
        try {
          const submittedEmployeeId = document.getElementById("staff-id").value.trim();
          const submittedFullName = document.getElementById("full-name").value.trim();
          const assetCodes = Array.from(selectedAssets.keys());

          const response = await fetch("/api/borrow-requests", {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({
              submittedEmployeeId,
              submittedFullName,
              assetCodes,
            }),
          });

          const payload = await response.json();
          if (!response.ok) {
            setMessage(payload.error || "Submit failed.");
            return;
          }

          selectedAssets.clear();
          renderSelected();
          setMessage(`Request ${payload.requestKey} submitted. IT will review it shortly.`, true);
        } catch (error) {
          setMessage(error instanceof Error ? error.message : "Submit failed.");
        }
      });
    </script>
  </body>
</html>"#
}
