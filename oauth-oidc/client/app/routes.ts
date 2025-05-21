import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/home.tsx"),
  route("/oauth2", "routes/oauth2/index.tsx"),
  route("/oauth2/callback", "routes/oauth2/callback.tsx"),
  route("/oauth2/clients", "routes/oauth2/clients.tsx"),
] satisfies RouteConfig;
