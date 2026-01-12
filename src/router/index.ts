import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";
import ConsolePage from "../pages/ConsolePage.vue";
import LLMSettingsPage from "../pages/LLMSettingsPage.vue";
import ToolSettingsPage from "../pages/ToolSettingsPage.vue";
import SecuritySettingsPage from "../pages/SecuritySettingsPage.vue";

const routes: RouteRecordRaw[] = [
  {
    path: "/",
    name: "console",
    component: ConsolePage,
  },
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
