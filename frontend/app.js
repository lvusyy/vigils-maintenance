const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;
const DEFAULT_ENDPOINT = "https://vigils.oocup.de/desktop-updates/{{target}}-{{arch}}/{{current_version}}.json";

const state = {
  installation: null,
  update: null,
  package: null,
  busy: false,
  logs: [],
  endpoint: localStorage.getItem("uvigils.endpoint") || DEFAULT_ENDPOINT,
};

const $ = (id) => document.getElementById(id);
const controls = [
  "refreshButton", "launchButton", "updateButton", "installUpdateButton", "openFolderButton",
  "repairButton", "uninstallButton", "choosePackageButton", "installLocalButton",
];

function setBusy(busy, title, detail) {
  state.busy = busy;
  controls.forEach((id) => { const el = $(id); if (el) el.disabled = busy; });
  if (title) $("activityTitle").textContent = title;
  if (detail) $("activityDetail").textContent = detail;
  $("activityIcon").textContent = busy ? "\uE895" : "\uE946";
  if (!busy) renderStatus();
}

function renderStatus() {
  const item = state.installation;
  if (!item) return;
  const installed = item.installed;
  $("statusMark").className = `status-mark ${installed ? "installed" : "absent"}`;
  $("statusMark").innerHTML = `<span class="mdl">${installed ? "&#xE73E;" : "&#xE7BA;"}</span>`;
  $("statusTitle").textContent = installed ? "Vigils 已安装" : "Vigils 未安装";
  $("statusDetail").textContent = installed
    ? `${item.displayName || "Vigils"} ${item.version || "版本未知"}`
    : "可从本地安装包或受信任的发布通道安装";
  $("pageSubtitle").textContent = installed ? "本机安装可维护" : "等待安装来源";
  $("installBadge").className = `badge ${installed ? "good" : "warn"}`;
  $("installBadge").textContent = installed ? "INSTALLED" : "NOT INSTALLED";
  $("platformValue").textContent = item.platform || "-";
  $("installTypeValue").textContent = item.installType || "-";
  $("versionValue").textContent = item.version || "-";
  $("locationValue").textContent = item.installLocation || item.appExecutable || "-";
  $("locationValue").title = $("locationValue").textContent;
  $("processValue").textContent = item.runningProcesses.length ? item.runningProcesses.join(", ") : "未运行";
  $("dataValue").textContent = item.dataLocation || "-";
  $("dataValue").title = $("dataValue").textContent;
  $("launchButton").disabled = state.busy || !item.appExecutable;
  $("openFolderButton").disabled = state.busy || !installed;
  $("repairButton").disabled = state.busy || !item.hubExecutable;
  $("uninstallButton").disabled = state.busy || !item.uninstallSupported;
  $("updateButton").disabled = state.busy;
  $("updateButton").querySelector("span:last-child").textContent = state.update?.available ? "有可用更新" : "检查更新";
  $("installLocalButton").disabled = state.busy || !state.package;
}

function renderRelease() {
  const update = state.update;
  $("releaseEmpty").classList.toggle("hidden", Boolean(update));
  $("releaseDetails").classList.toggle("hidden", !update);
  if (!update) return;
  $("releaseVersion").textContent = `v${update.version}`;
  $("releaseDate").textContent = update.pubDate ? new Date(update.pubDate).toLocaleDateString("zh-CN") : "日期未知";
  $("releaseNotes").textContent = update.notes || "此版本未提供发布说明。";
  $("securityBadge").className = `badge ${update.signaturePresent || update.sha256 ? "good" : "warn"}`;
  $("securityBadge").textContent = update.signaturePresent ? "MINISIGN" : "SHA-256";
  $("releaseState").textContent = update.available ? "可安装新版本" : "当前已是最新版本";
  $("releaseState").style.color = update.available ? "var(--green)" : "var(--muted)";
  $("installUpdateButton").disabled = state.busy || !update.available;
  $("installUpdateButton").classList.toggle("hidden", !update.available);
}

function renderPackage() {
  const item = state.package;
  $("packageFacts").classList.toggle("hidden", !item);
  $("packageName").textContent = item?.fileName || "未选择安装包";
  $("packageMeta").textContent = item
    ? item.path
    : `支持 ${(state.installation?.supportedPackages || []).join(" / ") || "当前系统原生安装包"}`;
  if (item) {
    $("packageType").textContent = item.packageType;
    $("packageSize").textContent = formatBytes(item.sizeBytes);
    $("packageHash").textContent = item.sha256;
    $("packageHash").title = item.sha256;
  }
  $("installLocalButton").disabled = state.busy || !item;
}

function renderLogs() {
  const root = $("operationLog");
  if (!state.logs.length) {
    root.innerHTML = '<div class="log-empty">本次会话尚无维护操作</div>';
    return;
  }
  root.innerHTML = state.logs.map((entry) => `
    <div class="log-entry">
      <time>${escapeHtml(entry.time)}</time>
      <span class="mdl ${entry.ok ? "ok" : "fail"}">${entry.ok ? "&#xE73E;" : "&#xEA39;"}</span>
      <code>${escapeHtml(entry.name)}</code>
      <p>${escapeHtml(entry.detail)}</p>
    </div>`).join("");
}

function addLog(name, ok, detail) {
  state.logs.unshift({ name, ok, detail: String(detail || ""), time: new Date().toLocaleTimeString("zh-CN", { hour12: false }) });
  renderLogs();
}

function logResult(result) {
  addLog("result", result.ok, result.message);
  [...result.steps].reverse().forEach((step) => addLog(step.name, step.ok, step.detail));
}

async function refreshStatus() {
  setBusy(true, "正在检测安装状态", "读取注册表、安装目录和进程状态");
  try {
    state.installation = await invoke("get_status");
    $("activityTitle").textContent = "状态已刷新";
    $("activityDetail").textContent = state.installation.installed ? "检测到可维护的 Vigils 安装" : "未检测到 Vigils 安装";
  } catch (error) {
    addLog("status", false, error);
    $("activityTitle").textContent = "状态检测失败";
    $("activityDetail").textContent = String(error);
  } finally {
    setBusy(false);
  }
}

async function checkUpdate() {
  setBusy(true, "正在检查更新", "连接受信任的发布通道");
  try {
    state.update = await invoke("check_update", { endpoint: state.endpoint });
    renderRelease();
    addLog("check-update", true, `${state.update.currentVersion} -> ${state.update.version}`);
    $("activityTitle").textContent = state.update.available ? "发现可用更新" : "当前已是最新版本";
    $("activityDetail").textContent = state.update.available ? `Vigils ${state.update.version}` : `Vigils ${state.update.currentVersion}`;
  } catch (error) {
    state.update = null;
    renderRelease();
    $("securityBadge").className = "badge warn";
    $("securityBadge").textContent = "UNAVAILABLE";
    addLog("check-update", false, error);
    $("activityTitle").textContent = "更新检查失败";
    $("activityDetail").textContent = String(error);
  } finally {
    setBusy(false);
  }
}

async function choosePackage() {
  try {
    const path = await invoke("choose_installer");
    if (!path) return;
    setBusy(true, "正在检查安装包", path);
    state.package = await invoke("inspect_installer", { path });
    renderPackage();
    addLog("inspect-package", true, `${state.package.fileName} SHA-256 ${state.package.sha256}`);
    $("activityTitle").textContent = "安装包校验完成";
    $("activityDetail").textContent = state.package.fileName;
  } catch (error) {
    addLog("inspect-package", false, error);
    $("activityTitle").textContent = "安装包无效";
    $("activityDetail").textContent = String(error);
  } finally {
    setBusy(false);
  }
}

async function installLocal() {
  if (!state.package) return;
  setBusy(true, "正在安装 Vigils", state.package.fileName);
  try {
    const result = await invoke("install_local", {
      path: state.package.path,
      expectedSha256: state.package.sha256,
      silent: $("silentInstall").checked,
      launchAfter: $("launchAfter").checked,
    });
    logResult(result);
    $("activityTitle").textContent = result.ok ? "安装完成" : "安装未完成";
    $("activityDetail").textContent = result.message;
  } catch (error) {
    addLog("install-local", false, error);
    $("activityTitle").textContent = "安装失败";
    $("activityDetail").textContent = String(error);
  } finally {
    setBusy(false);
    await refreshStatus();
  }
}

async function installUpdate() {
  setBusy(true, "正在下载更新", `准备 Vigils ${state.update?.version || ""}`);
  $("progressWrap").classList.remove("hidden");
  try {
    const result = await invoke("update_now", {
      endpoint: state.endpoint,
      silent: $("silentInstall").checked,
      launchAfter: true,
    });
    logResult(result);
    $("activityTitle").textContent = result.ok ? "更新完成" : "更新未完成";
    $("activityDetail").textContent = result.message;
  } catch (error) {
    addLog("update", false, error);
    $("activityTitle").textContent = "更新失败";
    $("activityDetail").textContent = String(error);
  } finally {
    setBusy(false);
    setTimeout(() => $("progressWrap").classList.add("hidden"), 1200);
    await refreshStatus();
  }
}

async function uninstallVigils() {
  setBusy(true, "正在卸载 Vigils", "停止运行时并还原接入配置");
  try {
    const result = await invoke("uninstall_vigils", {
      purgeData: $("purgeData").checked,
      force: $("forceUninstall").checked,
    });
    logResult(result);
    $("activityTitle").textContent = result.ok ? "卸载完成" : "卸载未完成";
    $("activityDetail").textContent = result.message;
  } catch (error) {
    addLog("uninstall", false, error);
    $("activityTitle").textContent = "卸载失败";
    $("activityDetail").textContent = String(error);
  } finally {
    setBusy(false);
    await refreshStatus();
  }
}

async function repair() {
  setBusy(true, "正在修复接入配置", "调用 vigil-hub setup --all");
  try {
    const result = await invoke("repair_integrations");
    logResult(result);
    $("activityTitle").textContent = result.ok ? "修复完成" : "修复失败";
    $("activityDetail").textContent = result.message;
  } catch (error) {
    addLog("repair", false, error);
  } finally {
    setBusy(false);
  }
}

function switchView(name) {
  document.querySelectorAll(".nav-item").forEach((item) => item.classList.toggle("active", item.dataset.view === name));
  document.querySelectorAll(".view").forEach((view) => view.classList.remove("active"));
  $(`${name}View`).classList.add("active");
  const labels = { overview: ["维护概览", "安装状态、版本和维护操作"], packages: ["安装来源", "选择并校验本地发布包"], history: ["执行记录", "本次维护会话的逐步结果"] };
  $("pageTitle").textContent = labels[name][0];
  $("pageSubtitle").textContent = labels[name][1];
}

function formatBytes(value) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 ** 2) return `${(value / 1024).toFixed(1)} KiB`;
  return `${(value / 1024 ** 2).toFixed(1)} MiB`;
}

function escapeHtml(value) {
  return String(value).replace(/[&<>'"]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;" })[ch]);
}

document.querySelectorAll(".nav-item").forEach((item) => item.addEventListener("click", () => switchView(item.dataset.view)));
$("refreshButton").addEventListener("click", refreshStatus);
$("updateButton").addEventListener("click", checkUpdate);
$("installUpdateButton").addEventListener("click", installUpdate);
$("choosePackageButton").addEventListener("click", choosePackage);
$("installLocalButton").addEventListener("click", installLocal);
$("repairButton").addEventListener("click", repair);
$("launchButton").addEventListener("click", () => invoke("launch_vigils").catch((error) => addLog("launch", false, error)));
$("openFolderButton").addEventListener("click", () => invoke("open_install_location").catch((error) => addLog("open-folder", false, error)));
$("uninstallButton").addEventListener("click", () => $("uninstallDialog").showModal());
$("confirmUninstallButton").addEventListener("click", (event) => { event.preventDefault(); $("uninstallDialog").close(); uninstallVigils(); });
$("settingsButton").addEventListener("click", () => { $("endpointInput").value = state.endpoint; $("settingsDialog").showModal(); });
$("resetEndpointButton").addEventListener("click", () => { $("endpointInput").value = DEFAULT_ENDPOINT; });
$("saveEndpointButton").addEventListener("click", (event) => {
  event.preventDefault();
  const value = $("endpointInput").value.trim();
  if (!value.startsWith("https://")) { addLog("settings", false, "更新源必须使用 HTTPS"); return; }
  state.endpoint = value;
  localStorage.setItem("uvigils.endpoint", value);
  state.update = null;
  renderRelease();
  $("settingsDialog").close();
  addLog("settings", true, "更新源已保存");
});
$("clearLogButton").addEventListener("click", () => { state.logs = []; renderLogs(); });

listen("maintenance-progress", (event) => {
  const data = event.payload;
  $("progressWrap").classList.remove("hidden");
  const percent = data.percent ?? 0;
  $("progressBar").style.width = `${percent}%`;
  $("progressText").textContent = data.percent == null ? formatBytes(data.downloaded) : `${percent}%`;
  $("activityDetail").textContent = data.phase === "verified" ? "签名与哈希校验通过" : `已下载 ${formatBytes(data.downloaded)}${data.total ? ` / ${formatBytes(data.total)}` : ""}`;
});

refreshStatus();
