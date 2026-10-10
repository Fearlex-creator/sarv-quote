import { createRoot } from "react-dom/client";
import App from "./app/App";
import { initialLocale } from "./lib/i18n";
import { ErrorBoundary } from "./components/ui/error-boundary";
import { TooltipProvider } from "./components/ui/tooltip";
import "./styles/globals.css";
const locale = initialLocale();
document.documentElement.lang = locale;
document.documentElement.dir = locale === "fa" ? "rtl" : "ltr";

createRoot(document.getElementById("root")!).render(
  <ErrorBoundary>
    <TooltipProvider>
      <App />
    </TooltipProvider>
  </ErrorBoundary>,
);
