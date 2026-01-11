import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";
import ConsolePage from "../pages/ConsolePage.vue";
import LLMSettingsPage from "../pages/LLMSettingsPage.vue";

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
    path: "/:pathMatch(.*)*",
    redirect: "/",
  },
];

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export default router;
