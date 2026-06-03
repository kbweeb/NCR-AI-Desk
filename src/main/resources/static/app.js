const messagesEl = document.getElementById("messages");
const emptyStateEl = document.getElementById("empty-state");
const form = document.getElementById("ask-form");
const input = document.getElementById("message");
const sendBtn = document.getElementById("send-btn");
const statusEl = document.getElementById("status");
const docInput = document.getElementById("doc-input");
const attachBtn = document.getElementById("attach-btn");
const docChip = document.getElementById("doc-chip");
const newChatBtn = document.getElementById("new-chat-btn");
const menuBtn = document.getElementById("menu-btn");
const sidebar = document.querySelector(".sidebar");
const chatListEl = document.getElementById("chat-list");
const chatHistoryEmptyEl = document.getElementById("chat-history-empty");
const topbarTitleEl = document.getElementById("topbar-title");

const USER_ID_KEY = "ncr-desk-user-id";
const ACTIVE_CHAT_KEY = "ncr-desk-active-chat-id";

const HEALTH_TIMEOUT_MS = 6000;
const ASK_TIMEOUT_MS = 130000;
const MAX_UPLOAD_BYTES = 10 * 1024 * 1024;
const HEALTH_POLL_MS = 20000;
const HEALTH_BOOTSTRAP_MS = 2500;

let chatHistory = [];
let activeChatId = null;
let activeSessionId = null;
let activeDocumentId = null;
let activeDocumentName = null;
let chatSummaries = [];
let deskOnline = false;
let aiOnline = false;
let healthPollTimer = null;
let firstHealthCheck = true;
let bootstrapping = false;

function getUserId() {
  let id = localStorage.getItem(USER_ID_KEY);
  if (!id) {
    id = crypto.randomUUID();
    localStorage.setItem(USER_ID_KEY, id);
  }
  return id;
}

function setActiveChatId(id) {
  activeChatId = id;
  if (id) {
    localStorage.setItem(ACTIVE_CHAT_KEY, id);
  } else {
    localStorage.removeItem(ACTIVE_CHAT_KEY);
  }
}

function getSessionId() {
  if (!activeSessionId) {
    activeSessionId = crypto.randomUUID();
  }
  return activeSessionId;
}

function renderMarkdownLight(text) {
  return escapeHtml(text)
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    .replace(/\n/g, "<br>");
}

function escapeHtml(s) {
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}

function downloadLabel(filename) {
  const lower = (filename || "").toLowerCase();
  if (lower.endsWith(".docx")) return "Download Word";
  if (lower.endsWith(".pdf")) return "Download PDF";
  if (lower.endsWith(".md")) return "Download Markdown";
  return "Download file";
}

function showChatView() {
  if (emptyStateEl) emptyStateEl.hidden = true;
  if (messagesEl) messagesEl.hidden = false;
}

function showEmptyView() {
  if (messagesEl) {
    messagesEl.innerHTML = "";
    messagesEl.hidden = true;
  }
  if (emptyStateEl) emptyStateEl.hidden = false;
}

function setComposerEnabled(enabled) {
  input.disabled = !enabled;
  attachBtn.disabled = !enabled;
  updateSendButton();
}

function updateSendButton() {
  const hasText = input.value.trim().length > 0;
  const canSend = deskOnline && hasText;
  sendBtn.disabled = !canSend;
  sendBtn.classList.toggle("ready", canSend);
}

function setStatus(text, state) {
  statusEl.textContent = text;
  statusEl.classList.remove("ok", "warn", "busy");
  if (state) statusEl.classList.add(state);
}

function fetchWithTimeout(url, options = {}, ms = HEALTH_TIMEOUT_MS) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), ms);
  return fetch(url, { ...options, signal: controller.signal }).finally(() =>
    clearTimeout(timer)
  );
}

async function checkHealth(silent = false) {
  if (!silent && firstHealthCheck) {
    setStatus("Connecting", "busy");
  }

  try {
    const res = await fetchWithTimeout("/api/health");
    if (res.status === 404) {
      deskOnline = false;
      aiOnline = false;
      setComposerEnabled(false);
      setStatus("UI outdated", "warn");
      setChatApiError("Spring API missing — run .\\stop.cmd then .\\start.cmd.");
      return;
    }
    if (!res.ok) throw new Error("unhealthy");
    const data = await res.json();

    deskOnline = data.backendReachable === true;
    aiOnline =
      data.aiAvailable === true ||
      data.backend?.llm?.liveAvailable === true ||
      data.backend?.llm?.qwenAvailable === true ||
      data.backend?.llm?.ollamaAvailable === true;

    firstHealthCheck = false;
    setComposerEnabled(deskOnline);

    if (!deskOnline) {
      setStatus("Offline", "warn");
      return;
    }
    if (aiOnline) {
      setStatus("Online", "ok");
    } else {
      setStatus("Directory mode", "warn");
    }
  } catch {
    deskOnline = false;
    aiOnline = false;
    setComposerEnabled(false);
    if (!silent || firstHealthCheck) {
      setStatus(firstHealthCheck ? "Starting…" : "Offline", "warn");
    }
  }
}

function startHealthPolling() {
  checkHealth(false);
  if (healthPollTimer) clearInterval(healthPollTimer);
  let bootstrapTicks = 0;
  const bootstrap = setInterval(() => {
    bootstrapTicks += 1;
    if (!deskOnline && bootstrapTicks < 12) {
      checkHealth(true);
    } else {
      clearInterval(bootstrap);
    }
  }, HEALTH_BOOTSTRAP_MS);
  healthPollTimer = setInterval(() => checkHealth(true), HEALTH_POLL_MS);
}

function appendMessage(role, html, extras) {
  showChatView();

  const wrap = document.createElement("div");
  wrap.className = `msg-wrap ${role === "user" ? "user" : "assistant"}`;

  const bubble = document.createElement("div");
  bubble.className = "bubble";
  bubble.innerHTML = html;

  if (extras?.preview) {
    const preview = document.createElement("pre");
    preview.className = "doc-preview";
    preview.textContent = extras.preview;
    bubble.appendChild(preview);
  }

  if (extras?.downloadUrl) {
    const actions = document.createElement("div");
    actions.className = "msg-actions";
    const link = document.createElement("a");
    link.className = "download-btn";
    link.href = extras.downloadUrl;
    link.download = extras.filename || "document";
    link.textContent = downloadLabel(extras.filename);
    actions.appendChild(link);
    bubble.appendChild(actions);
  }

  wrap.appendChild(bubble);
  messagesEl.appendChild(wrap);

  const scrollParent = messagesEl.closest(".chat-scroll") || messagesEl;
  scrollParent.scrollTop = scrollParent.scrollHeight;
  return wrap;
}

function buildDownloadUrl(documentId) {
  return `/api/documents/${encodeURIComponent(documentId)}/download?sessionId=${encodeURIComponent(getSessionId())}`;
}

function renderChatList() {
  if (!chatListEl) return;
  chatListEl.innerHTML = "";
  chatHistoryEmptyEl.hidden = chatSummaries.length > 0;

  for (const chat of chatSummaries) {
    const li = document.createElement("li");
    li.className = "chat-list-item";
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "chat-list-btn";
    if (chat.id === activeChatId) btn.classList.add("active");
    btn.dataset.chatId = chat.id;

    const title = document.createElement("span");
    title.className = "chat-list-title";
    title.textContent = chat.title || "New chat";

    const preview = document.createElement("span");
    preview.className = "chat-list-preview";
    preview.textContent = chat.preview || "No messages yet";

    btn.appendChild(title);
    btn.appendChild(preview);
    btn.addEventListener("click", () => selectChat(chat.id));
    li.appendChild(btn);
    chatListEl.appendChild(li);
  }
}

async function loadChatList() {
  try {
    const res = await fetchWithTimeout(
      `/api/chats?userId=${encodeURIComponent(getUserId())}`,
      {},
      8000
    );
    if (res.status === 404) {
      setChatApiError("Chat API not found — restart Spring (run .\\stop.cmd then .\\start.cmd).");
      return false;
    }
    if (!res.ok) return false;
    chatSummaries = await res.json();
    setChatApiError(null);
    renderChatList();
    return true;
  } catch {
    return false;
  }
}

function setChatApiError(msg) {
  if (!chatHistoryEmptyEl) return;
  if (msg) {
    chatHistoryEmptyEl.hidden = false;
    chatHistoryEmptyEl.textContent = msg;
  } else {
    chatHistoryEmptyEl.hidden = chatSummaries.length > 0;
    chatHistoryEmptyEl.textContent = "No chats yet";
  }
}

async function persistMessage(role, content) {
  if (!activeChatId || !content?.trim()) return;
  try {
    await fetch(`/api/chats/${encodeURIComponent(activeChatId)}/messages`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        userId: getUserId(),
        role,
        content: content.trim(),
      }),
    });
    await loadChatList();
  } catch {
    /* non-fatal */
  }
}

async function patchChatDocument() {
  if (!activeChatId) return;
  try {
    await fetch(`/api/chats/${encodeURIComponent(activeChatId)}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        userId: getUserId(),
        documentId: activeDocumentId || "",
        documentName: activeDocumentName || "",
      }),
    });
  } catch {
    /* non-fatal */
  }
}

function applyChatDetail(detail) {
  setActiveChatId(detail.id);
  activeSessionId = detail.sessionId;
  chatHistory = (detail.messages || []).map((m) => ({
    role: m.role,
    content: m.content,
  }));
  setActiveDocument(detail.documentId, detail.documentName);
  if (topbarTitleEl) {
    topbarTitleEl.textContent = detail.title || "NCR AI Desk";
  }

  messagesEl.innerHTML = "";
  if (chatHistory.length === 0) {
    showEmptyView();
  } else {
    showChatView();
    for (const turn of chatHistory) {
      const role = turn.role === "user" ? "user" : "assistant";
      appendMessage(role, renderMarkdownLight(turn.content));
    }
  }
  renderChatList();
  updateSendButton();
}

async function selectChat(chatId) {
  if (chatId === activeChatId) {
    closeSidebar();
    return;
  }
  try {
    const res = await fetch(
      `/api/chats/${encodeURIComponent(chatId)}?userId=${encodeURIComponent(getUserId())}`
    );
    if (!res.ok) throw new Error("load failed");
    const detail = await res.json();
    applyChatDetail(detail);
    closeSidebar();
  } catch {
    appendMessage("assistant", "Could not load that chat.");
  }
}

async function createNewChat() {
  try {
    const res = await fetch("/api/chats", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ userId: getUserId(), title: "New chat" }),
    });
    if (!res.ok) throw new Error("create failed");
    const detail = await res.json();
    await loadChatList();
    applyChatDetail(detail);
    closeSidebar();
    input.focus();
  } catch {
    setChatApiError("Could not create a chat. Run .\\stop.cmd then .\\start.cmd.");
  }
}

function setActiveDocument(id, name) {
  activeDocumentId = id || null;
  if (name) activeDocumentName = name;
  if (id) {
    if (name) patchChatDocument();
  } else {
    activeDocumentName = null;
    if (activeChatId) patchChatDocument();
  }
  updateDocChip();
}

function updateDocChip() {
  if (!docChip) return;
  if (activeDocumentId && activeDocumentName) {
    docChip.hidden = false;
    docChip.querySelector(".doc-chip-name").textContent = activeDocumentName;
  } else {
    docChip.hidden = true;
  }
}

async function uploadDocument(file) {
  if (!deskOnline) {
    appendMessage("assistant", "The desk is still starting. Try again in a moment.");
    return;
  }
  if (!activeChatId) {
    await createNewChat();
    if (!activeChatId) return;
  }

  if (file.size > MAX_UPLOAD_BYTES) {
    appendMessage("assistant", escapeHtml("File too large (max 10 MB)."));
    return;
  }

  const formData = new FormData();
  formData.append("sessionId", getSessionId());
  formData.append("file", file);

  appendMessage("user", escapeHtml(file.name));
  await persistMessage("user", `Attached: ${file.name}`);
  const typing = appendMessage("assistant", '<span class="typing-dots">Reading</span>');

  try {
    const res = await fetch("/api/documents/upload", {
      method: "POST",
      body: formData,
    });
    typing.remove();

    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      let msg = err.error || err.detail || err.message || "";
      if (
        !msg ||
        msg === "Internal Server Error" ||
        msg === "Bad Gateway"
      ) {
        msg =
          res.status === 413
            ? "File too large (max 10 MB)."
            : res.status >= 500
              ? "Upload failed on the server. Restart Spring (8080), API (8090), and the document service (8092), then try again."
              : "Could not read that file. Use PDF, Word (.docx), or text under 10 MB.";
      }
      appendMessage("assistant", escapeHtml(msg));
      await persistMessage("assistant", msg);
      return;
    }

    const data = await res.json();
    setActiveDocument(data.documentId, data.filename);
    const okMsg =
      `${data.filename} is attached (${data.pageCount} page${data.pageCount === 1 ? "" : "s"}, ${data.format}). ` +
      "Try: “Summarize this document”, “What are the key fields?”, or “Improve the wording and export as PDF”.";
    appendMessage("assistant", escapeHtml(okMsg));
    await persistMessage("assistant", okMsg);
  } catch {
    typing.remove();
    const msg = "Upload failed. Check that the API is running.";
    appendMessage("assistant", msg);
    await persistMessage("assistant", msg);
  }
}

async function ask(message) {
  if (!deskOnline) {
    appendMessage(
      "assistant",
      "Still connecting to the desk. Wait until the status shows Online or Directory mode."
    );
    return;
  }
  if (!activeChatId) {
    await createNewChat();
  }

  appendMessage("user", escapeHtml(message));
  chatHistory.push({ role: "user", content: message });
  await persistMessage("user", message);

  const typing = appendMessage("assistant", '<span class="typing-dots">Thinking</span>');
  sendBtn.disabled = true;

  try {
    const res = await fetchWithTimeout(
      "/api/ask",
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          message,
          sessionId: getSessionId(),
          history: chatHistory.slice(0, -1),
          documentId: activeDocumentId,
        }),
      },
      ASK_TIMEOUT_MS
    );
    typing.remove();

    if (!res.ok) {
      const errBody = await res.json().catch(() => ({}));
      const detail =
        errBody.error ||
        errBody.message ||
        (res.status === 502
          ? "The desk timed out waiting for the AI. Try again — short questions work even in Directory mode."
          : `Request failed (${res.status}).`);
      appendMessage("assistant", escapeHtml(detail));
      await persistMessage("assistant", detail);
      return;
    }

    const data = await res.json();
    chatHistory.push({ role: "assistant", content: data.reply });
    await persistMessage("assistant", data.reply);

    if (
      data.reply &&
      data.reply.includes("don't have that document in this session")
    ) {
      setActiveDocument(null, null);
    }

    if (data.activeDocumentId && !activeDocumentName) {
      setActiveDocument(data.activeDocumentId, "document");
    }

    const extras = {};
    if (data.documentEditPreview) extras.preview = data.documentEditPreview;
    if (data.documentArtifact) {
      extras.downloadUrl = buildDownloadUrl(data.documentArtifact.documentId);
      extras.filename = data.documentArtifact.filename;
    }

    appendMessage("assistant", renderMarkdownLight(data.reply), extras);
    await loadChatList();
  } catch {
    typing.remove();
    const msg =
      "Could not reach the desk. Confirm Spring (8080) and the Rust API (8090) are running.";
    appendMessage("assistant", msg);
    await persistMessage("assistant", msg);
  } finally {
    updateSendButton();
    input.focus();
  }
}

function submitMessage(text) {
  const trimmed = (text || input.value).trim();
  if (!trimmed) return;
  input.value = "";
  input.style.height = "auto";
  updateSendButton();
  closeSidebar();
  ask(trimmed);
}

function closeSidebar() {
  sidebar?.classList.remove("open");
  document.querySelector(".sidebar-overlay")?.classList.remove("visible");
}

function openSidebar() {
  sidebar?.classList.add("open");
  let overlay = document.querySelector(".sidebar-overlay");
  if (!overlay) {
    overlay = document.createElement("div");
    overlay.className = "sidebar-overlay";
    overlay.addEventListener("click", closeSidebar);
    document.body.appendChild(overlay);
  }
  overlay.classList.add("visible");
}

function autoResizeTextarea() {
  input.style.height = "auto";
  input.style.height = `${Math.min(input.scrollHeight, 200)}px`;
  updateSendButton();
}

async function waitForBackend(maxMs = 45000) {
  const start = Date.now();
  while (Date.now() - start < maxMs) {
    try {
      const res = await fetchWithTimeout("/api/health", {}, 4000);
      if (res.ok) {
        await checkHealth(true);
        if (deskOnline) return true;
      }
    } catch {
      /* retry */
    }
    await new Promise((r) => setTimeout(r, 1500));
  }
  return false;
}

async function bootstrapChats() {
  if (bootstrapping) return;
  bootstrapping = true;

  const apiOk = await loadChatList();
  if (!apiOk) {
    activeSessionId = crypto.randomUUID();
    setActiveChatId(null);
    showEmptyView();
    bootstrapping = false;
    return;
  }

  const savedId = localStorage.getItem(ACTIVE_CHAT_KEY);
  if (savedId && chatSummaries.some((c) => c.id === savedId)) {
    await selectChat(savedId);
  } else if (chatSummaries.length > 0) {
    await selectChat(chatSummaries[0].id);
  } else {
    await createNewChat();
  }
  bootstrapping = false;
}

async function initApp() {
  startHealthPolling();
  await waitForBackend();
  await bootstrapChats();
  autoResizeTextarea();
}

form.addEventListener("submit", (e) => {
  e.preventDefault();
  submitMessage();
});

input.addEventListener("input", autoResizeTextarea);
input.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    if (!sendBtn.disabled) submitMessage();
  }
});

document.querySelectorAll("[data-prompt]").forEach((el) => {
  el.addEventListener("click", () => {
    const prompt = el.getAttribute("data-prompt");
    if (prompt) submitMessage(prompt);
  });
});

if (attachBtn && docInput) {
  attachBtn.addEventListener("click", () => docInput.click());
  docInput.addEventListener("change", () => {
    const file = docInput.files?.[0];
    docInput.value = "";
    if (file) uploadDocument(file);
  });
}

docChip?.querySelector(".doc-chip-clear")?.addEventListener("click", () => {
  setActiveDocument(null, null);
});

newChatBtn?.addEventListener("click", () => createNewChat());
menuBtn?.addEventListener("click", openSidebar);

initApp();
