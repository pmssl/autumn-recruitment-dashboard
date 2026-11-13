import init from "./pkg/autumn_web.js";

const form = document.querySelector("#login");
const canvas = document.querySelector("#canvas");
const tokenInput = document.querySelector("#token");
const message = document.querySelector("#message");
const EVENT_LOOP_STARTED = "Using exceptions for control flow, don't mind me. This isn't actually an error!";

async function openDashboard(token) {
  const response = await fetch("/api/data", {
    headers: { Authorization: `Bearer ${token}` },
    cache: "no-store",
  });
  if (!response.ok) throw new Error(response.status === 401 ? "口令错误或服务尚未配置" : "云端数据暂时无法读取");
  sessionStorage.setItem("dashboard-token", token);
  form.style.display = "none";
  canvas.style.display = "block";
  try {
    await init();
  } catch (error) {
    // winit throws this after scheduling the browser event loop.
    if (!(error instanceof Error && error.message === EVENT_LOOP_STARTED)) throw error;
  }
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  message.textContent = "正在连接…";
  try {
    await openDashboard(tokenInput.value);
  } catch (error) {
    message.textContent = error.message || "打开失败";
    form.style.display = "block";
    canvas.style.display = "none";
  }
});

const existing = sessionStorage.getItem("dashboard-token");
if (existing) {
  openDashboard(existing).catch(() => {
    sessionStorage.removeItem("dashboard-token");
    message.textContent = "请重新输入访问口令";
  });
}
