import init from "./pkg/autumn_web.js";

const form = document.querySelector("#login");
const canvas = document.querySelector("#canvas");
const tokenInput = document.querySelector("#token");
const message = document.querySelector("#message");

async function openDashboard(token) {
  const response = await fetch("/api/data", {
    headers: { Authorization: `Bearer ${token}` },
    cache: "no-store",
  });
  if (!response.ok) throw new Error(response.status === 401 ? "口令错误或服务尚未配置" : "云端数据暂时无法读取");
  sessionStorage.setItem("dashboard-token", token);
  form.style.display = "none";
  canvas.style.display = "block";
  await init();
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
