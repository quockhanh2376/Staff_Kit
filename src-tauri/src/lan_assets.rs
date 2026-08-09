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

      #submit-button:disabled {
        background: var(--surface-2);
        border-color: var(--border);
        color: var(--muted);
        cursor: not-allowed;
        opacity: 0.7;
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

      .asset-code-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
      }

      .selected-indicator {
        display: inline-flex;
        flex: 0 0 auto;
        align-items: center;
        justify-content: center;
        width: 24px;
        height: 24px;
        border: 1px solid rgba(69, 212, 131, 0.5);
        border-radius: 999px;
        background: rgba(69, 212, 131, 0.16);
        color: var(--success);
        font-size: 16px;
        font-weight: 800;
        line-height: 1;
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

      .selected-card {
        display: flex;
        align-items: center;
        gap: 10px;
        margin-top: 10px;
        padding: 10px 12px;
        border: 1px solid rgba(69, 212, 131, 0.5);
        border-radius: 12px;
        background: rgba(69, 212, 131, 0.12);
      }

      .selected-card .check {
        color: var(--success);
        font-size: 18px;
        font-weight: 800;
      }

      .selected-card .details {
        flex: 1;
        min-width: 0;
      }

      .selected-card .canonical {
        font-weight: 700;
      }

      .selected-card .matched {
        margin-top: 2px;
        color: var(--muted);
        font-size: 12px;
      }

      .selected-card .availability {
        color: var(--success);
        font-size: 12px;
        font-weight: 700;
      }

      .selected-card .remove {
        width: auto;
        margin: 0;
        padding: 4px 8px;
        border: 0;
        background: transparent;
        color: var(--muted);
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

      .message.error {
        border: 1px solid rgba(248, 113, 113, 0.45);
        background: rgba(248, 113, 113, 0.12);
        color: #fecaca;
      }

      .confirmation-section {
        margin-top: 16px;
        padding: 14px;
        border: 1px solid var(--border);
        border-radius: 14px;
        background: var(--surface-2);
      }

      .confirmation-section h2 {
        margin: 0 0 10px;
        font-size: 16px;
      }

      .policy-block {
        margin-top: 8px;
        padding: 10px;
        border-radius: 10px;
        background: rgba(255, 255, 255, 0.04);
        color: var(--text);
        line-height: 1.45;
        white-space: pre-wrap;
      }

      .policy-language {
        margin: 0 0 4px;
        color: var(--muted);
        font-size: 12px;
        font-weight: 700;
      }

      .acknowledgment {
        display: flex;
        gap: 8px;
        align-items: flex-start;
        margin-top: 12px;
        font-size: 13px;
        line-height: 1.4;
      }

      .acknowledgment input {
        width: auto;
        margin-top: 2px;
      }

      .signature-label {
        margin-top: 12px;
        margin-bottom: 6px;
      }

      #signature-canvas {
        display: block;
        width: 100%;
        aspect-ratio: 2 / 1;
        min-height: 120px;
        max-height: 240px;
        border: 1px solid var(--border);
        border-radius: 10px;
        background: white;
        touch-action: none;
      }

      .confirmation-actions {
        display: flex;
        justify-content: flex-end;
        margin-top: 6px;
      }

      .clear-signature {
        width: auto;
        padding: 6px 10px;
        background: transparent;
        color: var(--muted);
      }

      .confirmation-hint {
        margin: 8px 0 0;
        font-size: 12px;
      }

      @media (max-width: 480px) {
        main { padding: 16px 10px 28px; }
        .card { padding: 14px; }
        .search-row { gap: 6px; }
        #search-btn { padding-inline: 12px; }
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
        <div id="confirmation-section"></div>
        <button id="submit-button" type="button">Submit Borrow Request</button>
        <div id="message"></div>
      </div>
    </main>

    <script>
      // State
      let mode = "borrow"; // "borrow" | "return"
      const selectedAssets = new Map();
      let borrowPolicy = null;
      let policyLoading = false;
      let policyError = "";
      let policyAcknowledged = false;
      let typedName = "";
      let signatureStrokeCount = 0;
      let signatureInkPresent = false;
      let signatureCanvas = null;
      let isDrawing = false;
      let currentStrokeHasInk = false;
      let strokeStartPoint = null;

      // Element refs
      const resultsEl   = document.getElementById("asset-results");
      const selectedEl  = document.getElementById("selected-assets");
      const confirmationEl = document.getElementById("confirmation-section");
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

      // LAN pages are commonly opened over plain HTTP from a phone. The
      // randomUUID() API is restricted to secure contexts, while
      // getRandomValues() remains available for this non-secret identifier.
      const createRequestId = () => {
        const cryptoApi = globalThis.crypto;
        if (typeof cryptoApi?.randomUUID === "function") {
          return cryptoApi.randomUUID();
        }
        if (typeof cryptoApi?.getRandomValues === "function") {
          const bytes = new Uint8Array(16);
          cryptoApi.getRandomValues(bytes);
          bytes[6] = (bytes[6] & 0x0f) | 0x40;
          bytes[8] = (bytes[8] & 0x3f) | 0x80;
          return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0"))
            .join("")
            .replace(/^(.{8})(.{4})(.{4})(.{4})(.{12})$/, "$1-$2-$3-$4-$5");
        }
        return `lan-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
      };

      const clientSessionId = createRequestId();
      let isSearching = false;
      let isSubmitting = false;

      const normalizedTypedName = () => typedName.trim().replace(/\\s+/g, " ");
      const hasSignatureEvidence = () => signatureStrokeCount > 0 && signatureInkPresent;
      const hasConfirmationEvidence = () => Boolean(normalizedTypedName()) || hasSignatureEvidence();
      const getCanvasContext = (canvas) => {
        try { return canvas?.getContext?.("2d") || null; } catch (_error) { return null; }
      };

      const clearSignature = () => {
        signatureStrokeCount = 0;
        signatureInkPresent = false;
        currentStrokeHasInk = false;
        strokeStartPoint = null;
        if (signatureCanvas) {
          const context = getCanvasContext(signatureCanvas);
          if (context) {
            context.clearRect(0, 0, signatureCanvas.width, signatureCanvas.height);
            context.fillStyle = "white";
            context.fillRect(0, 0, signatureCanvas.width, signatureCanvas.height);
          }
        }
      };

      const resetEvidence = () => {
        policyAcknowledged = false;
        typedName = "";
        clearSignature();
      };

      const updateSubmitState = () => {
        const policyReady = mode === "return" || Boolean(borrowPolicy);
        const acknowledged = mode === "return" || policyAcknowledged;
        submitBtn.disabled = isSubmitting || selectedAssets.size === 0 || !policyReady || !acknowledged || !hasConfirmationEvidence();
      };

      const initializeSignatureCanvas = () => {
        if (!signatureCanvas || signatureCanvas.dataset.initialized === "true") return;
        signatureCanvas.dataset.initialized = "true";
        const context = getCanvasContext(signatureCanvas);
        const resize = () => {
          const ratio = globalThis.devicePixelRatio || 1;
          const width = Math.max(signatureCanvas.clientWidth || 480, 240);
          const height = Math.round(width / 2);
          signatureCanvas.width = Math.round(width * ratio);
          signatureCanvas.height = Math.round(height * ratio);
          if (context) {
            context.setTransform(ratio, 0, 0, ratio, 0, 0);
            context.fillStyle = "white";
            context.fillRect(0, 0, width, height);
            context.strokeStyle = "rgb(17, 24, 39)";
            context.lineWidth = 2;
            context.lineCap = "round";
            context.lineJoin = "round";
          }
        };
        resize();
        const point = (event) => {
          const rect = signatureCanvas.getBoundingClientRect();
          return { x: event.clientX - rect.left, y: event.clientY - rect.top };
        };
        signatureCanvas.addEventListener("pointerdown", (event) => {
          event.preventDefault();
          isDrawing = true;
          const pointValue = point(event);
          currentStrokeHasInk = false;
          strokeStartPoint = pointValue;
          context?.beginPath();
          context?.moveTo(pointValue.x, pointValue.y);
        });
        signatureCanvas.addEventListener("pointermove", (event) => {
          if (!isDrawing) return;
          event.preventDefault();
          const pointValue = point(event);
          if (!currentStrokeHasInk && strokeStartPoint && Math.hypot(pointValue.x - strokeStartPoint.x, pointValue.y - strokeStartPoint.y) >= 2) {
            currentStrokeHasInk = true;
            signatureStrokeCount += 1;
            signatureInkPresent = true;
            updateSubmitState();
          }
          context?.lineTo(pointValue.x, pointValue.y);
          context?.stroke();
        });
        const stopDrawing = () => { isDrawing = false; currentStrokeHasInk = false; strokeStartPoint = null; };
        signatureCanvas.addEventListener("pointerup", stopDrawing);
        signatureCanvas.addEventListener("pointercancel", stopDrawing);
        signatureCanvas.addEventListener("pointerleave", stopDrawing);
        window.addEventListener("resize", resize);
      };

      const renderConfirmation = () => {
        confirmationEl.replaceChildren();
        if (selectedAssets.size === 0) {
          updateSubmitState();
          return;
        }
        const section = document.createElement("section");
        section.id = "confirmation-section-content";
        section.className = "confirmation-section";
        const heading = document.createElement("h2");
        heading.textContent = mode === "borrow" ? "Handle with Care" : "Return Confirmation";
        section.append(heading);

        if (mode === "borrow") {
          if (policyLoading) {
            const loading = document.createElement("p");
            loading.className = "helper";
            loading.textContent = "Loading the current Handle with Care policy...";
            section.append(loading);
          } else if (policyError) {
            const error = document.createElement("p");
            error.className = "message error";
            error.textContent = policyError;
            section.append(error);
          } else if (borrowPolicy) {
            const enLabel = document.createElement("div");
            enLabel.className = "policy-language";
            enLabel.textContent = "English";
            const en = document.createElement("div");
            en.id = "policy-english";
            en.className = "policy-block";
            en.textContent = borrowPolicy.textEn;
            const viLabel = document.createElement("div");
            viLabel.className = "policy-language";
            viLabel.textContent = "Tiếng Việt";
            const vi = document.createElement("div");
            vi.id = "policy-vietnamese";
            vi.className = "policy-block";
            vi.textContent = borrowPolicy.textVi;
            section.append(enLabel, en, viLabel, vi);
            const acknowledgment = document.createElement("label");
            acknowledgment.className = "acknowledgment";
            const checkbox = document.createElement("input");
            checkbox.id = "acknowledgment-checkbox";
            checkbox.type = "checkbox";
            checkbox.checked = policyAcknowledged;
            checkbox.addEventListener("change", () => {
              policyAcknowledged = checkbox.checked;
              updateSubmitState();
            });
            const checkboxText = document.createElement("span");
            checkboxText.textContent = "I have carefully read and agree to the above.\nTôi đã đọc kỹ và đồng ý với nội dung trên.";
            acknowledgment.append(checkbox, checkboxText);
            section.append(acknowledgment);
          }
        } else {
          const en = document.createElement("div");
          en.id = "return-confirmation-en";
          en.className = "policy-block";
          en.textContent = "I confirm that I am returning the device(s) listed above to IT.";
          const vi = document.createElement("div");
          vi.id = "return-confirmation-vi";
          vi.className = "policy-block";
          vi.textContent = "Tôi xác nhận đang bàn giao lại cho IT các thiết bị được liệt kê ở trên.";
          section.append(en, vi);
        }

        const signatureLabel = document.createElement("div");
        signatureLabel.className = "signature-label";
        signatureLabel.textContent = "Signature (optional if typing your full name)";
        section.append(signatureLabel);
        const canvas = document.createElement("canvas");
        canvas.id = "signature-canvas";
        canvas.setAttribute("aria-label", "Handwritten signature");
        signatureCanvas = canvas;
        section.append(canvas);
        const signatureActions = document.createElement("div");
        signatureActions.className = "confirmation-actions";
        const clearButton = document.createElement("button");
        clearButton.type = "button";
        clearButton.className = "clear-signature";
        clearButton.textContent = "Clear signature";
        clearButton.addEventListener("click", () => { clearSignature(); updateSubmitState(); });
        signatureActions.append(clearButton);
        section.append(signatureActions);
        const typedLabel = document.createElement("label");
        typedLabel.className = "signature-label";
        typedLabel.setAttribute("for", "typed-name");
        typedLabel.textContent = "Type your full name (optional if signing)";
        section.append(typedLabel);
        const typedInput = document.createElement("input");
        typedInput.id = "typed-name";
        typedInput.autocomplete = "name";
        typedInput.value = typedName;
        typedInput.addEventListener("input", () => { typedName = typedInput.value; updateSubmitState(); });
        section.append(typedInput);
        const hint = document.createElement("p");
        hint.className = "helper confirmation-hint";
        hint.textContent = "Use a handwritten signature, your typed full name, or both.";
        section.append(hint);
        confirmationEl.append(section);
        initializeSignatureCanvas();
        updateSubmitState();
      };

      const loadBorrowPolicy = async () => {
        if (mode !== "borrow") return;
        policyLoading = true;
        policyError = "";
        renderConfirmation();
        try {
          const res = await authorizedFetch("/api/borrow-policy");
          const payload = await res.json().catch(() => null);
          if (!res.ok || !payload || !Number.isInteger(payload.version) || typeof payload.textEn !== "string" || !payload.textEn.trim() || typeof payload.textVi !== "string" || !payload.textVi.trim()) {
            throw new Error("policy");
          }
          if (borrowPolicy && borrowPolicy.version !== payload.version) resetEvidence();
          borrowPolicy = { version: payload.version, textEn: payload.textEn, textVi: payload.textVi };
        } catch (_error) {
          resetEvidence();
          borrowPolicy = null;
          policyError = "The Handle with Care policy could not be loaded. Please try again.";
        } finally {
          policyLoading = false;
          renderConfirmation();
        }
      };

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

        // Clear state when switching mode because eligibility changes.
        clearInteractionState();
        if (mode === "borrow") loadBorrowPolicy();
      }

      btnBorrow.addEventListener("click", () => applyMode("borrow"));
      btnReturn.addEventListener("click", () => applyMode("return"));

      // Render helpers
      const renderSelected = () => {
        selectedEl.replaceChildren();
        if (selectedAssets.size === 0) {
          renderConfirmation();
          return;
        }
        const heading = document.createElement("div");
        heading.className = "helper";
        heading.textContent = "Selected Assets";
        const cards = document.createElement("div");
        for (const asset of selectedAssets.values()) {
          const card = document.createElement("div");
          card.className = "selected-card";
          const check = document.createElement("span");
          check.className = "check";
          check.textContent = "\u2713";
          const details = document.createElement("div");
          details.className = "details";
          const canonical = document.createElement("div");
          canonical.className = "canonical";
          canonical.textContent = asset.assetCode;
          const matched = document.createElement("div");
          matched.className = "matched";
          matched.textContent = asset.matchedIdentifier
            ? `Matched: ${asset.matchedIdentifier}`
            : `${asset.assetType} - ${asset.displayName}`;
          const availability = document.createElement("div");
          availability.className = "availability";
          availability.textContent = mode === "return" ? "Eligible · Assigned" : "Eligible · In Stock";
          details.append(canonical, matched, availability);
          const remove = document.createElement("button");
          remove.type = "button";
          remove.className = "remove";
          remove.dataset.remove = asset.assetCode;
          remove.textContent = "×";
          remove.addEventListener("click", () => {
            selectedAssets.delete(asset.assetCode);
            resetEvidence();
            signatureCanvas = null;
            renderSelected();
          });
          card.append(check, details, remove);
          cards.append(card);
        }
        selectedEl.append(heading, cards);
        renderConfirmation();
      };

      updateSubmitState();

      const setMessage = (text, isSuccess = false) => {
        messageEl.replaceChildren();
        if (!text) return;
        const message = document.createElement("div");
        message.className = isSuccess ? "message success" : "message error";
        message.textContent = text;
        messageEl.append(message);
      };

      const sanitizedSubmitError = (payload) => {
        const code = typeof payload?.error === "string" ? payload.error.toLowerCase() : "";
        if (code.includes("policy") || code.includes("acknowledg")) return "The policy has changed. Reload it and acknowledge it again.";
        if (code.includes("typed") || code.includes("name")) return "The typed name must match the submitted employee name.";
        if (code.includes("signature") || code.includes("png") || code.includes("ink")) return "Please provide a valid, non-blank signature.";
        if (code.includes("size") || code.includes("body")) return "The confirmation is too large. Please use a smaller signature.";
        if (code.includes("asset") || code.includes("stock") || code.includes("loan") || code.includes("claim")) return "The selected asset is no longer available. Search again.";
        if (code.includes("borrower")) return "The selected return assets must belong to the same borrower.";
        return "Submit failed. Please review the form and try again.";
      };

      const clearInteractionState = () => {
        selectedAssets.clear();
        resetEvidence();
        signatureCanvas = null;
        renderSelected();
        resultsEl.replaceChildren();
        setMessage("");
      };

      const renderResults = (items, searchTerm) => {
        resultsEl.replaceChildren();
        if (!Array.isArray(items) || items.length === 0) {
          const empty = document.createElement("div");
          empty.className = "helper";
          empty.textContent = mode === "return"
            ? "No eligible assigned assets matched. Check the identifier and try again."
            : "No eligible in-stock assets matched. Check the identifier and try again.";
          resultsEl.append(empty);
          return;
        }
        const isReturn = mode === "return";
        for (const asset of items) {
          const item = document.createElement("div");
          item.className = "asset-item";
          const code = document.createElement("div");
          code.className = "asset-code-row";
          const strong = document.createElement("strong");
          strong.textContent = asset.assetCode;
          code.append(strong);
          const description = document.createElement("div");
          description.textContent = `${asset.assetType} - ${asset.displayName}`;
          const metadata = document.createElement("div");
          metadata.className = "helper";
          metadata.textContent = `${asset.model ?? ""} ${asset.serialNumber ?? ""}`;
          const alreadySelected = selectedAssets.has(asset.assetCode);
          if (alreadySelected) {
            const indicator = document.createElement("span");
            indicator.className = "selected-indicator";
            indicator.setAttribute("aria-label", "Selected asset");
            indicator.title = "Selected asset";
            indicator.textContent = "\u2713";
            code.append(indicator);
          } else {
            const add = document.createElement("button");
            add.type = "button";
            add.className = isReturn ? "return-mode" : "";
            add.dataset.add = asset.assetCode;
            add.textContent = MODES[mode].addLabel;
            add.addEventListener("click", () => {
              selectedAssets.set(asset.assetCode, { ...asset, matchedIdentifier: searchTerm });
              resetEvidence();
              renderSelected();
              setMessage(`\u2713 ${asset.assetCode} selected and eligible for ${mode}.`, true);
            });
            item.append(add);
          }
          item.prepend(code, description, metadata);
          resultsEl.append(item);
        }
      };

      // -- Asset search --------------------------------------------------------
      const doSearch = async () => {
        if (isSearching) return;
        isSearching = true;
        searchBtn.disabled = true;
        const searchTerm = searchInput.value.trim();
        try {
          const q = encodeURIComponent(searchTerm);
          const res = await authorizedFetch(`${MODES[mode].endpoint}?q=${q}`);
          const responsePayload = await res.json().catch(() => null);
          if (!res.ok) {
            selectedAssets.clear();
            resetEvidence();
            renderSelected();
            resultsEl.replaceChildren();
            setMessage("Asset search failed. Please try again.");
            return;
          }
          const items = Array.isArray(responsePayload)
            ? responsePayload.filter((asset) => asset && typeof asset.assetCode === "string" && asset.assetCode.trim())
            : [];
          if (items.length === 0) {
            selectedAssets.clear();
            resetEvidence();
            renderSelected();
          }
          const autoSelected = items.length === 1;
          if (autoSelected) {
            resetEvidence();
            selectedAssets.set(items[0].assetCode, {
              ...items[0],
              matchedIdentifier: searchTerm,
            });
            renderSelected();
          }
          renderResults(items, searchTerm);
          if (items.length > 0) {
            setMessage(autoSelected
              ? `\u2713 Found and selected ${items[0].assetCode}. Ready to submit.`
              : `\u2713 Found ${items.length} eligible assets. Select one below.`, true);
          } else {
            setMessage(mode === "return"
              ? "No eligible assigned asset matched. Verify the identifier and try again."
              : "No eligible in-stock asset matched. Verify the identifier and try again.");
          }
        } catch (err) {
          selectedAssets.clear();
          resetEvidence();
          renderSelected();
          resultsEl.replaceChildren();
          setMessage(err instanceof Error && err.message.includes("token")
            ? err.message
            : "Asset search failed. Please try again.");
        } finally {
          isSearching = false;
          searchBtn.disabled = false;
        }
      };

      searchBtn.addEventListener("click", doSearch);
      searchInput.addEventListener("keydown", (e) => { if (e.key === "Enter") doSearch(); });

      // Submit
      submitBtn.addEventListener("click", async () => {
        if (isSubmitting || submitBtn.disabled || selectedAssets.size === 0) return;
        isSubmitting = true;
        submitBtn.disabled = true;
        try {
          const submittedEmployeeId = document.getElementById("staff-id").value.trim();
          const submittedFullName   = document.getElementById("full-name").value.trim();
          const assetCodes          = Array.from(selectedAssets.keys());
          const signaturePngBase64 = hasSignatureEvidence() && signatureCanvas?.toDataURL
            ? signatureCanvas.toDataURL("image/png")
            : null;
          const confirmation = {
            policyVersion: mode === "borrow" ? borrowPolicy?.version ?? null : null,
            policyAcknowledged: mode === "borrow" ? policyAcknowledged : false,
            confirmationMethod: hasSignatureEvidence() && normalizedTypedName() ? "both" : hasSignatureEvidence() ? "signature" : "typed_name",
            signaturePngBase64,
            signatureStrokeCount: hasSignatureEvidence() ? signatureStrokeCount : null,
            signatureInkPresent: hasSignatureEvidence() ? signatureInkPresent : null,
            typedName: normalizedTypedName() || null,
            assetCodesSnapshot: assetCodes,
          };

          const res = await authorizedFetch("/api/borrow-requests", {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({
              submittedEmployeeId,
              submittedFullName,
              assetCodes,
              requestType: mode,
              requestId: createRequestId(),
              clientSessionId,
              confirmation,
            }),
          });

          const responsePayload = await res.json().catch(() => null);
          if (!res.ok) {
            const safeError = sanitizedSubmitError(responsePayload);
            if (safeError.includes("policy has changed")) {
              resetEvidence();
              signatureCanvas = null;
              await loadBorrowPolicy();
            }
            setMessage(safeError);
            return;
          }

          selectedAssets.clear();
          resetEvidence();
          signatureCanvas = null;
          renderSelected();
          const verb = mode === "return" ? "Return" : "Borrow";
          resultsEl.replaceChildren();
          searchInput.value = "";
          setMessage(`${verb} request ${responsePayload.requestReference} submitted. ${responsePayload.message}`, true);
        } catch (err) {
          setMessage("Submit failed. Please check your connection and try again.");
        } finally {
          isSubmitting = false;
          updateSubmitState();
        }
      });

      loadBorrowPolicy();

      // A mobile browser may restore this page from its back-forward cache.
      // Do not carry an old selection into a new visible interaction session.
      window.addEventListener("pageshow", (event) => {
        if (event.persisted) clearInteractionState();
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

        assert!(html.contains("const createRequestId = () =>"));
        assert!(html.contains("const clientSessionId = createRequestId()"));
        assert!(html.contains("requestId: createRequestId()"));
        assert!(html.contains("clientSessionId"));
    }

    #[test]
    fn borrow_page_requires_bilingual_confirmation_and_sends_evidence() {
        let html = borrow_page_html();
        let script = r###"
import { JSDOM } from "jsdom";
import fs from "node:fs";

const page = fs.readFileSync(0, "utf8");
const calls = [];
const asset = { assetCode: "VNLAP326", assetType: "Laptop", displayName: "ASWVNLAP326", model: null, serialNumber: null };
const wait = () => new Promise((resolve) => setTimeout(resolve, 20));
const dom = new JSDOM(page, {
  url: "http://192.168.2.1:8787/borrow#t=browser-token",
  runScripts: "dangerously",
  beforeParse(window) {
    window.fetch = async (input, init = {}) => {
      const url = String(input);
      calls.push({ url, init });
      if (url.includes("/api/borrow-policy")) {
        return { ok: true, json: async () => ({ version: 7, textEn: "Handle with care.", textVi: "Vui lòng giữ gìn thiết bị." }) };
      }
      if (url.includes("/api/borrow-requests")) {
        return { ok: true, json: async () => ({ requestReference: "BR-PHASE4", message: "Pending IT review." }) };
      }
      return { ok: true, json: async () => [asset] };
    };
  },
});
const document = dom.window.document;
const submit = document.getElementById("submit-button");
document.getElementById("asset-search").value = "ASWVNLAP326";
document.getElementById("search-btn").click();
await wait();
if (!document.querySelector("#policy-english")?.textContent.includes("Handle with care") ||
    !document.querySelector("#policy-vietnamese")?.textContent.includes("Vui lòng") ||
    !document.querySelector("#acknowledgment-checkbox")) throw new Error("borrow policy was not rendered as separate bilingual blocks");
if (!submit.disabled) throw new Error("submit enabled before acknowledgment evidence");
document.getElementById("acknowledgment-checkbox").click();
document.getElementById("typed-name").value = "Nguyễn Văn A";
document.getElementById("typed-name").dispatchEvent(new dom.window.Event("input", { bubbles: true }));
if (submit.disabled) throw new Error("typed-name confirmation did not enable submit");
submit.click();
await wait();
const post = calls.find((call) => call.url.includes("/api/borrow-requests"));
const payload = JSON.parse(post.init.body);
if (payload.confirmation.policyVersion !== 7 || payload.confirmation.policyAcknowledged !== true ||
    payload.confirmation.confirmationMethod !== "typed_name" || payload.confirmation.typedName !== "Nguyễn Văn A" ||
    JSON.stringify(payload.confirmation.assetCodesSnapshot) !== JSON.stringify(["VNLAP326"]) ||
    "textEn" in payload.confirmation || "textVi" in payload.confirmation) throw new Error("borrow confirmation payload was incorrect");
document.getElementById("btn-return").click();
document.getElementById("asset-search").value = "ASWVNLAP326";
document.getElementById("search-btn").click();
await wait();
if (document.querySelector("#confirmation-section")?.textContent.includes("Handle with care") ||
    !document.querySelector("#return-confirmation-en")?.textContent.includes("returning")) throw new Error("return confirmation/policy mode was incorrect");
console.log("phase4-confirmation-ok");
"###;

        let mut child = Command::new("node")
            .args(["--input-type=module", "-e", script])
            .current_dir(std::env::current_dir().expect("workspace directory"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("node and jsdom are required for Phase 4 browser coverage");
        child
            .stdin
            .take()
            .expect("node stdin")
            .write_all(html.as_bytes())
            .expect("write page HTML");
        let output = child.wait_with_output().expect("wait for Phase 4 browser test");
        assert!(
            output.status.success(),
            "Phase 4 browser flow failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("phase4-confirmation-ok"));
    }

    #[test]
    fn borrow_page_supports_signature_only_and_resets_evidence_without_storage() {
        let html = borrow_page_html();
        let script = r###"
import { JSDOM } from "jsdom";
import fs from "node:fs";

const page = fs.readFileSync(0, "utf8");
const calls = [];
const asset = { assetCode: "VNLAP326", assetType: "Laptop", displayName: "ASWVNLAP326", model: null, serialNumber: null };
const wait = () => new Promise((resolve) => setTimeout(resolve, 20));
const context = { setTransform() {}, fillRect() {}, clearRect() {}, beginPath() {}, moveTo() {}, lineTo() {}, stroke() {} };
const dom = new JSDOM(page, {
  url: "http://192.168.2.1:8787/borrow#t=browser-token",
  runScripts: "dangerously",
  beforeParse(window) {
    window.HTMLCanvasElement.prototype.getContext = () => context;
    window.HTMLCanvasElement.prototype.toDataURL = () => "data:image/png;base64,UE5H";
    window.fetch = async (input, init = {}) => {
      const url = String(input);
      calls.push({ url, init });
      if (url.includes("/api/borrow-policy")) return { ok: true, json: async () => ({ version: 2, textEn: "Care.", textVi: "Giữ gìn." }) };
      if (url.includes("/api/borrow-requests")) return { ok: true, json: async () => ({ requestReference: "BR-SIGN", message: "Pending." }) };
      return { ok: true, json: async () => [asset] };
    };
  },
});
const document = dom.window.document;
const pointer = (type, x, y) => {
  const event = new dom.window.Event(type, { bubbles: true, cancelable: true });
  Object.defineProperties(event, { clientX: { value: x }, clientY: { value: y } });
  return event;
};
document.getElementById("asset-search").value = "ASWVNLAP326";
document.getElementById("search-btn").click();
await wait();
document.getElementById("acknowledgment-checkbox").click();
const canvas = document.getElementById("signature-canvas");
canvas.dispatchEvent(pointer("pointerdown", 2, 2));
canvas.dispatchEvent(pointer("pointermove", 8, 5));
canvas.dispatchEvent(pointer("pointerup", 8, 5));
if (document.getElementById("submit-button").disabled) throw new Error("signature did not enable submit");
document.querySelector(".clear-signature").click();
if (!document.getElementById("submit-button").disabled) throw new Error("clear did not invalidate signature evidence");
canvas.dispatchEvent(pointer("pointerdown", 2, 2));
canvas.dispatchEvent(pointer("pointermove", 8, 5));
canvas.dispatchEvent(pointer("pointerup", 8, 5));
document.getElementById("submit-button").click();
await wait();
const post = calls.find((call) => call.url.includes("/api/borrow-requests"));
const payload = JSON.parse(post.init.body);
if (payload.confirmation.confirmationMethod !== "signature" ||
    !payload.confirmation.signaturePngBase64?.startsWith("data:image/png;base64,") ||
    payload.confirmation.typedName !== null || payload.confirmation.signatureStrokeCount !== 1 ||
    dom.window.localStorage.length !== 0 || dom.window.sessionStorage.length !== 0) throw new Error("signature evidence contract failed");
document.getElementById("btn-return").click();
document.getElementById("asset-search").value = "ASWVNLAP326";
document.getElementById("search-btn").click();
await wait();
if (document.querySelector("#policy-english") || !document.querySelector("#return-confirmation-vi")) throw new Error("return rendered Borrow policy");
console.log("signature-reset-ok");
"###;

        let mut child = Command::new("node")
            .args(["--input-type=module", "-e", script])
            .current_dir(std::env::current_dir().expect("workspace directory"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("node and jsdom are required for signature coverage");
        child
            .stdin
            .take()
            .expect("node stdin")
            .write_all(html.as_bytes())
            .expect("write page HTML");
        let output = child.wait_with_output().expect("wait for signature test");
        assert!(
            output.status.success(),
            "signature flow failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("signature-reset-ok"));
    }

    #[test]
    fn borrow_page_starts_empty_and_does_not_retain_interaction_state() {
        let html = borrow_page_html();
        assert!(!html.contains("ASWVNLAP326"));
        assert!(!html.contains("VNLAP326"));

        let script = r###"
import { JSDOM } from "jsdom";
import fs from "node:fs";

const page = fs.readFileSync(0, "utf8");
const asset = { assetCode: "VNLAP326", assetType: "Laptop", displayName: "ASWVNLAP326", model: null, serialNumber: null };
const wait = () => new Promise((resolve) => setTimeout(resolve, 20));
const dom = new JSDOM(page, {
  url: "http://192.168.2.1:8787/borrow#t=browser-token",
  runScripts: "dangerously",
  beforeParse(window) {
    Object.defineProperty(window.crypto, "randomUUID", { value: () => "state-test-id" });
    window.fetch = async (input, init = {}) => {
      if (String(input).includes("/api/borrow-policy")) {
        return { ok: true, json: async () => ({ version: 1, textEn: "Handle with care.", textVi: "Vui lòng giữ gìn thiết bị." }) };
      }
      if (init.method === "POST") {
        return { ok: true, json: async () => ({ requestReference: "BR-STATE", message: "Pending IT review." }) };
      }
      return { ok: true, json: async () => String(input).includes("ASWVNLAP326") ? [asset] : [] };
    };
  },
});
const document = dom.window.document;
const selected = document.getElementById("selected-assets");
const results = document.getElementById("asset-results");
const submit = document.getElementById("submit-button");
const search = document.getElementById("asset-search");
const searchButton = document.getElementById("search-btn");
if (selected.textContent || results.childElementCount !== 0 || !submit.disabled || document.body.textContent.includes("VNLAP326")) {
  throw new Error("fresh LAN page was not empty");
}

search.value = "ASWVNLAP326";
searchButton.click();
await wait();
if (!selected.textContent.includes("VNLAP326") || !submit.disabled) {
  throw new Error("successful search did not select the asset or keep confirmation required");
}

document.getElementById("btn-return").click();
if (selected.textContent || results.childElementCount !== 0 || !submit.disabled) {
  throw new Error("mode switch retained borrow state");
}

document.getElementById("btn-borrow").click();
search.value = "ASWVNLAP326";
searchButton.click();
await wait();
document.getElementById("staff-id").value = "1301";
document.getElementById("full-name").value = "Test user";
document.getElementById("acknowledgment-checkbox").click();
document.getElementById("typed-name").value = "Test user";
document.getElementById("typed-name").dispatchEvent(new dom.window.Event("input", { bubbles: true }));
submit.click();
await wait();
if (selected.textContent || results.childElementCount !== 0 || !submit.disabled || !document.querySelector("#message .success")) {
  throw new Error("successful submit did not clear interaction state");
}

search.value = "ASWVNLAP326";
searchButton.click();
await wait();
search.value = "MISSING-ASSET";
searchButton.click();
await wait();
if (selected.textContent || !submit.disabled) {
  throw new Error("failed search resurrected stale selection");
}

search.value = "ASWVNLAP326";
searchButton.click();
await wait();
const pageshow = new dom.window.Event("pageshow");
Object.defineProperty(pageshow, "persisted", { value: true });
dom.window.dispatchEvent(pageshow);
if (selected.textContent || results.childElementCount !== 0 || !submit.disabled) {
  throw new Error("restored LAN page retained prior selection");
}
console.log("state-reset-ok");
"###;

        let mut child = Command::new("node")
            .args(["--input-type=module", "-e", script])
            .current_dir(std::env::current_dir().expect("workspace directory"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("node and jsdom are required for LAN state coverage");
        child
            .stdin
            .take()
            .expect("node stdin")
            .write_all(html.as_bytes())
            .expect("write page HTML");
        let output = child
            .wait_with_output()
            .expect("wait for LAN state test");
        assert!(
            output.status.success(),
            "LAN state flow failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("state-reset-ok"));
    }

    #[test]
    fn borrow_page_prevents_duplicate_submissions_and_surfaces_api_errors() {
        let html = borrow_page_html();

        assert!(html.contains("let isSubmitting = false"));
        assert!(html.contains("submitBtn.disabled = true"));
        assert!(html.contains("if (!res.ok)"));
        assert!(html.contains("sanitizedSubmitError(responsePayload)"));
    }

    #[test]
    fn borrow_page_executes_shared_borrow_and_return_flow_with_bearer_auth() {
        let html = borrow_page_html();
        let script = r###"
import { JSDOM } from "jsdom";
import fs from "node:fs";

const page = fs.readFileSync(0, "utf8");
const calls = [];
const asset = { assetCode: "ASSET-001", assetType: "Laptop", displayName: "Demo Laptop", model: "Model X", serialNumber: "SERIAL-001" };
let uuid = 0;
const wait = () => new Promise((resolve) => setTimeout(resolve, 20));
const dom = new JSDOM(page, {
  url: "http://127.0.0.1/borrow#t=browser-token",
  runScripts: "dangerously",
  beforeParse(window) {
    Object.defineProperty(window.crypto, "randomUUID", { value: () => `id-${++uuid}` });
    window.fetch = async (input, init = {}) => {
      calls.push({ input: String(input), init: { ...init, headers: new Headers(init.headers) } });
      if (String(input).includes("/api/borrow-policy")) {
        return { ok: true, json: async () => ({ version: 1, textEn: "Handle with care.", textVi: "Vui lòng giữ gìn thiết bị." }) };
      }
      if (String(input).includes("/api/borrow-requests")) {
        return { ok: true, json: async () => ({ requestReference: "BR-0001", message: "Pending IT review." }) };
      }
      return { ok: true, json: async () => [asset] };
    };
  },
});
const document = dom.window.document;
if (dom.window.location.hash) throw new Error("token fragment remained visible");
const auth = (call) => call.init.headers.get("Authorization");
const search = document.getElementById("asset-search");
const searchButton = document.getElementById("search-btn");
const submit = document.getElementById("submit-button");
search.value = "ASSET-001";
searchButton.click();
await wait();
const borrowSearchCall = calls.find((call) => call.input.includes("/api/assets"));
if (!borrowSearchCall || auth(borrowSearchCall) !== "Bearer browser-token") throw new Error("borrow search auth failed");
document.getElementById("staff-id").value = "EE1001";
document.getElementById("full-name").value = "Client name is not authoritative";
document.getElementById("acknowledgment-checkbox").click();
document.getElementById("typed-name").value = "Client name is not authoritative";
document.getElementById("typed-name").dispatchEvent(new dom.window.Event("input", { bubbles: true }));
submit.click();
submit.click();
await wait();
const borrowCalls = calls.filter((call) => call.input.includes("/api/borrow-requests"));
const borrowPayload = JSON.parse(borrowCalls[0].init.body);
if (borrowCalls.length !== 1 || borrowPayload.requestType !== "borrow" || borrowPayload.clientSessionId !== "id-1") throw new Error("borrow flow contract failed");
if (auth(borrowCalls[0]) !== "Bearer browser-token") throw new Error("borrow submit auth failed");

document.getElementById("btn-return").click();
search.value = "ASSET-001";
searchButton.click();
await wait();
if (!calls.some((call) => call.input.includes("/api/assigned-assets") && auth(call) === "Bearer browser-token")) throw new Error("return search auth failed");
document.getElementById("typed-name").value = "Return operator";
document.getElementById("typed-name").dispatchEvent(new dom.window.Event("input", { bubbles: true }));
submit.click();
await wait();
const submitCalls = calls.filter((call) => call.input.includes("/api/borrow-requests"));
const returnPayload = JSON.parse(submitCalls[1].init.body);
if (returnPayload.requestType !== "return" || returnPayload.clientSessionId !== "id-1") throw new Error("return flow contract failed");
if (returnPayload.requestId === borrowPayload.requestId) throw new Error("requestId was reused");
if (auth(submitCalls[1]) !== "Bearer browser-token") throw new Error("return submit auth failed");
console.log("shared-flow-ok");
"###;

        let mut child = Command::new("node")
            .args(["--input-type=module", "-e", script])
            .current_dir(std::env::current_dir().expect("workspace directory"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("node and jsdom are required for browser flow coverage");
        child
            .stdin
            .take()
            .expect("node stdin")
            .write_all(html.as_bytes())
            .expect("write page HTML");
        let output = child
            .wait_with_output()
            .expect("wait for browser flow test");
        assert!(
            output.status.success(),
            "browser flow failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("shared-flow-ok"));
    }

    #[test]
    fn borrow_page_makes_derived_search_results_obvious_and_submits_canonical_code() {
        let html = borrow_page_html();
        let script = r###"
import { JSDOM } from "jsdom";
import fs from "node:fs";

const page = fs.readFileSync(0, "utf8");
const calls = [];
const asset = { assetCode: "VNLAP326", assetType: "Laptop", displayName: "Dell Latitude 5540", model: null, serialNumber: null };
const wait = () => new Promise((resolve) => setTimeout(resolve, 20));
const dom = new JSDOM(page, {
  url: "http://127.0.0.1/borrow#t=browser-token",
  runScripts: "dangerously",
  beforeParse(window) {
    Object.defineProperty(window.crypto, "randomUUID", { value: () => "search-test-id" });
    window.fetch = async (input, init = {}) => {
      const url = String(input);
      calls.push({ url, init });
      if (url.includes("/api/borrow-policy")) {
        return { ok: true, json: async () => ({ version: 1, textEn: "Handle with care.", textVi: "Vui lòng giữ gìn thiết bị." }) };
      }
      if (url.includes("/api/borrow-requests")) {
        return { ok: true, json: async () => ({ requestReference: "BR-0002", message: "Pending IT review." }) };
      }
      if (url.includes("FAIL")) {
        return { ok: false, json: async () => ({ error: "raw database error should not render" }) };
      }
      if (url.includes("ASWVNLAP326")) {
        return { ok: true, json: async () => [asset] };
      }
      return { ok: true, json: async () => [] };
    };
  },
});
const document = dom.window.document;
const search = document.getElementById("asset-search");
const searchButton = document.getElementById("search-btn");
const submit = document.getElementById("submit-button");
if (!submit.disabled) throw new Error("submit must begin disabled");

search.value = "ASWVNLAP326";
searchButton.click();
await wait();
const success = document.querySelector("#message .success");
if (!success || !success.textContent.includes("✓")) throw new Error("search success was not obvious");
const result = document.querySelector(".asset-item");
if (!result || !result.textContent.includes("VNLAP326")) {
  throw new Error("derived search response did not map to canonical selectable asset");
}
const selectedIndicator = result.querySelector(".selected-indicator");
if (!selectedIndicator || selectedIndicator.getAttribute("aria-label") !== "Selected asset" || result.querySelector("[data-add]")) {
  throw new Error("selected asset did not render as a compact accessible indicator");
}
if (!submit.disabled) throw new Error("submit enabled before confirmation evidence");
if (!document.querySelector(".selected-card")?.textContent.includes("VNLAP326")) {
  throw new Error("selected asset card was not rendered");
}
document.getElementById("staff-id").value = "1301";
document.getElementById("full-name").value = "Test user";
document.getElementById("acknowledgment-checkbox").click();
document.getElementById("typed-name").value = "Test user";
document.getElementById("typed-name").dispatchEvent(new dom.window.Event("input", { bubbles: true }));
submit.click();
await wait();
const submitCall = calls.find((call) => call.url.includes("/api/borrow-requests"));
const payload = JSON.parse(submitCall.init.body);
if (payload.assetCodes.length !== 1 || payload.assetCodes[0] !== "VNLAP326") {
  throw new Error("submission did not use canonical asset code");
}

search.value = "MISSING-ASSET";
searchButton.click();
await wait();
if (!submit.disabled || document.querySelector(".selected-card")) {
  throw new Error("failed search left stale selected asset");
}
if (!document.querySelector("#message .error")?.textContent.includes("No eligible")) {
  throw new Error("no-match feedback was not actionable");
}

search.value = "FAIL";
searchButton.click();
await wait();
const error = document.querySelector("#message .error");
if (!error || !error.textContent.includes("Please try again") || error.textContent.includes("raw database")) {
  throw new Error("failed search feedback was not sanitized");
}
console.log("selection-feedback-ok");
"###;

        let mut child = Command::new("node")
            .args(["--input-type=module", "-e", script])
            .current_dir(std::env::current_dir().expect("workspace directory"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("node and jsdom are required for selection feedback coverage");
        child
            .stdin
            .take()
            .expect("node stdin")
            .write_all(html.as_bytes())
            .expect("write page HTML");
        let output = child
            .wait_with_output()
            .expect("wait for selection feedback test");
        assert!(
            output.status.success(),
            "selection feedback flow failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("selection-feedback-ok"));
    }

    #[test]
    fn borrow_page_searches_on_insecure_lan_origins_without_random_uuid() {
        let html = borrow_page_html();
        let script = r###"
import { JSDOM } from "jsdom";
import fs from "node:fs";

const page = fs.readFileSync(0, "utf8");
const calls = [];
const wait = () => new Promise((resolve) => setTimeout(resolve, 20));
const dom = new JSDOM(page, {
  url: "http://192.168.2.1:8787/borrow#t=browser-token",
  runScripts: "dangerously",
  beforeParse(window) {
    Object.defineProperty(window.crypto, "randomUUID", { value: undefined, configurable: true });
    window.fetch = async (input, init = {}) => {
      const url = String(input);
      calls.push({ input: url, init });
      if (url.includes("/api/borrow-policy")) {
        return { ok: true, json: async () => ({ version: 1, textEn: "Handle with care.", textVi: "Vui lòng giữ gìn thiết bị." }) };
      }
      return { ok: true, json: async () => [{
        assetCode: "VNLAP326",
        assetType: "Laptop",
        displayName: "Demo Laptop",
        model: null,
        serialNumber: null,
      }] };
    };
  },
});
const document = dom.window.document;
document.getElementById("asset-search").value = "ASWVNLAP326";
document.getElementById("search-btn").click();
await wait();
if (calls.filter((call) => !call.input.includes("/api/borrow-policy")).length !== 1 || !document.querySelector(".selected-card")) {
  throw new Error("insecure-origin search handler did not run");
}
console.log("insecure-origin-search-ok");
"###;

        let mut child = Command::new("node")
            .args(["--input-type=module", "-e", script])
            .current_dir(std::env::current_dir().expect("workspace directory"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("node and jsdom are required for insecure-origin browser coverage");
        child
            .stdin
            .take()
            .expect("node stdin")
            .write_all(html.as_bytes())
            .expect("write page HTML");
        let output = child
            .wait_with_output()
            .expect("wait for insecure-origin browser test");
        assert!(
            output.status.success(),
            "insecure-origin browser flow failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("insecure-origin-search-ok"));
    }

    #[test]
    fn borrow_page_handles_malformed_non_success_json_without_html_or_raw_leakage() {
        let html = borrow_page_html();
        let script = r###"
import { JSDOM } from "jsdom";
import fs from "node:fs";

const page = fs.readFileSync(0, "utf8");
const maliciousError = '<script>window.__xss=1</script>';
const wait = () => new Promise((resolve) => setTimeout(resolve, 20));
const dom = new JSDOM(page, {
  url: "http://127.0.0.1/borrow#t=browser-token",
  runScripts: "dangerously",
  beforeParse(window) {
    window.__xss = 0;
    Object.defineProperty(window.crypto, "randomUUID", { value: () => "test-id" });
    window.fetch = async (input) => {
      if (String(input).includes("/api/borrow-policy")) {
        return { ok: true, json: async () => ({ version: 1, textEn: "Handle with care.", textVi: "Vui lòng giữ gìn thiết bị." }) };
      }
      if (String(input).includes("/api/borrow-requests")) {
        return { ok: false, json: async () => { throw new Error(maliciousError); } };
      }
      return { ok: true, json: async () => [{ assetCode: "ASSET-001", assetType: "Laptop", displayName: "Demo", model: null, serialNumber: null }] };
    };
  },
});
const document = dom.window.document;
document.getElementById("search-btn").click();
await wait();
document.getElementById("acknowledgment-checkbox").click();
document.getElementById("typed-name").value = "Test Employee";
document.getElementById("typed-name").dispatchEvent(new dom.window.Event("input", { bubbles: true }));
document.getElementById("submit-button").click();
await wait();
const message = document.getElementById("message");
if (message.textContent !== "Submit failed. Please review the form and try again.") throw new Error("unexpected malformed-response message");
if (message.textContent.includes(maliciousError) || message.querySelector("script")) throw new Error("raw malformed error leaked");
if (dom.window.__xss !== 0) throw new Error("malformed error executed");
console.log("malformed-response-safe");
"###;

        let mut child = Command::new("node")
            .args(["--input-type=module", "-e", script])
            .current_dir(std::env::current_dir().expect("workspace directory"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("node and jsdom are required for malformed response coverage");
        child
            .stdin
            .take()
            .expect("node stdin")
            .write_all(html.as_bytes())
            .expect("write page HTML");
        let output = child
            .wait_with_output()
            .expect("wait for malformed response test");
        assert!(
            output.status.success(),
            "malformed response flow failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("malformed-response-safe"));
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
    window.fetch = async (input, init = {}) => {
      if (String(input).includes("/api/borrow-policy")) {
        return { ok: true, json: async () => ({ version: 1, textEn: "Handle with care.", textVi: "Vui lòng giữ gìn thiết bị." }) };
      }
      return init.method === "POST"
        ? { ok: false, json: async () => ({ error: payloads.error }) }
        : { ok: true, json: async () => [asset] };
    };
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
const selected = document.querySelector("#selected-assets");
if (!selected.textContent.includes(payloads.assetCode) || selected.querySelector("img,script,svg")) {
  throw new Error("selected asset became markup");
}
document.getElementById("staff-id").value = "EE1001";
document.getElementById("full-name").value = "Test Employee";
document.getElementById("acknowledgment-checkbox").click();
document.getElementById("typed-name").value = "Test Employee";
document.getElementById("typed-name").dispatchEvent(new dom.window.Event("input", { bubbles: true }));
document.getElementById("submit-button").click();
await wait();
const message = document.getElementById("message");
if (!message.textContent.includes("Submit failed") || message.textContent.includes(payloads.error) || message.querySelector("img,script,svg")) {
  throw new Error("backend error was not sanitized");
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
