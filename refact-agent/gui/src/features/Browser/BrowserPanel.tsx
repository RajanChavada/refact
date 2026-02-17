import classNames from "classnames";
import { useAppSelector } from "../../hooks";
import { selectBrowserRuntime } from "./browserSlice";
import { BrowserToolbar } from "./BrowserToolbar";
import styles from "./Browser.module.css";

type BrowserPanelProps = {
  chatId: string;
};

export const BrowserPanel = ({ chatId }: BrowserPanelProps) => {
  const runtime = useAppSelector((state) =>
    selectBrowserRuntime(state, chatId),
  );

  const isConnected = runtime?.connected ?? false;
  const url = runtime?.url ?? "";
  const frame = runtime?.latest_frame;

  return (
    <div className={styles.browserPanel}>
      <BrowserToolbar chatId={chatId} />
      <div className={styles.statusBar}>
        <span
          className={classNames(styles.statusDot, {
            [styles.statusDotConnected]: isConnected,
            [styles.statusDotDisconnected]: !isConnected,
          })}
        />
        <span className={styles.statusUrl}>
          {url || (isConnected ? "Connected" : "Not connected")}
        </span>
      </div>
      {frame && (
        <div className={styles.frameContainer}>
          <img
            className={styles.frameImage}
            src={`data:${frame.mime};base64,${frame.data}`}
            alt="Browser frame"
          />
        </div>
      )}
      {!frame && isConnected && (
        <div className={styles.frameContainer}>
          <span className={styles.framePlaceholder}>
            Waiting for browser frame…
          </span>
        </div>
      )}
    </div>
  );
};
