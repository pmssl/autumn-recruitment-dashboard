import { deriveDashboardToken } from "./cloudflare.mjs";

console.log(deriveDashboardToken(process.env.CLOUDFLARE_API_TOKEN));
