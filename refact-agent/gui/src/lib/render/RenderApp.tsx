import { StrictMode } from "react";
import { type Config } from "../../features/Config/configSlice";
import { App } from "../../features/App";
import { reportBuddyFrontendError } from "../../features/Buddy/reportBuddyFrontendError";
import { withBuddyErrorReport } from "../../features/Buddy/BuddyErrorBoundary";
import ReactDOM from "react-dom/client";
import "./web.css";

export function renderApp(element: HTMLElement, config: Config) {
  const AppWrapped: React.FC<Config> = () => {
    return (
      <StrictMode>
        <App />
      </StrictMode>
    );
  };

  const root = withBuddyErrorReport(
    () =>
      ReactDOM.createRoot(element, {
        onRecoverableError(error) {
          void reportBuddyFrontendError({
            source: "react_recoverable",
            error,
            sourceFile: "frontend/react_recoverable",
            toolName: "react_recoverable",
          });
        },
      }),
    {
      source: "react_root_render",
      sourceFile: "frontend/react_root_create",
      toolName: "react_root_create",
    },
  );

  withBuddyErrorReport(() => root.render(<AppWrapped {...config} />), {
    source: "react_root_render",
    sourceFile: "frontend/react_root_render",
    toolName: "react_root_render",
  });
}
