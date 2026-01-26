import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";
import ConsolePage from "../pages/ConsolePage.vue";
import LLMSettingsPage from "../pages/LLMSettingsPage.vue";
import ToolSettingsPage from "../pages/ToolSettingsPage.vue";
import SecuritySettingsPage from "../pages/SecuritySettingsPage.vue";

const cockpitRoutes: RouteRecordRaw[] = [
  { path: "/", name: "console", component: ConsolePage, alias: "/console" },
  { path: "/mission", name: "mission", component: ConsolePage },
  { path: "/plan", name: "plan", component: ConsolePage },
  { path: "/loop", name: "loop", component: ConsolePage },
  { path: "/terminal", name: "terminal", component: ConsolePage },
  { path: "/diff", name: "diff", component: ConsolePage },
  { path: "/git", name: "git", component: ConsolePage },
  { path: "/timeline", name: "timeline", component: ConsolePage },
];

const routes: RouteRecordRaw[] = [
  ...cockpitRoutes,
  {
    path: "/settings",
    name: "llm",
    component: LLMSettingsPage,
  },
  {
    path: "/settings/tools",
    name: "tools",
    component: ToolSettingsPage,
  },
  {
    path: "/settings/security",
    name: "security",
    component: SecuritySettingsPage,
  },
  {
    path: "/:pathMatch(.*)*",
    redirect: "/",
  },
];

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export default router;
