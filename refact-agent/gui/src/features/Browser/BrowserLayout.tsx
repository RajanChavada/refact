import { type ReactNode } from "react";
import { BrowserPanel } from "./BrowserPanel";
import styles from "./Browser.module.css";

type BrowserLayoutProps = {
  chatId: string;
  children: ReactNode;
};

export const BrowserLayout = ({ chatId, children }: BrowserLayoutProps) => {
  return (
    <div className={styles.browserLayout}>
      <BrowserPanel chatId={chatId} />
      <div className={styles.chatArea}>{children}</div>
    </div>
  );
};
