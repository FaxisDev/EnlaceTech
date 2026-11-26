export type Status =
  | "pending"
  | "available"
  | "unavailable"
  | "missing"
  | "downloading"
  | "completed"
  | "failed";

export interface LinkItem {
  id: string;
  url: string;
  fileId: string;
  name: string | null;
  size: number | null;
  status: Status;
  /** Why it cannot be downloaded, in Spanish. */
  reason: string | null;
  /** The message exactly as Rapidgator sent it. */
  rawReason: string | null;
  addedAt: number;
  checkedAt: number | null;
  savedPath: string | null;
  checking: boolean;
  received: number;
  speed: number;
}

export interface Account {
  email: string;
  isPremium: boolean;
  /** Unix seconds. */
  premiumEnd: number | null;
  trafficLeft: number | null;
  trafficTotal: number | null;
}

export interface Snapshot {
  links: LinkItem[];
  downloadDir: string;
  defaultDir: boolean;
  intervalMinutes: number;
  rememberPassword: boolean;
  /** Send a system notification when a link becomes downloadable. */
  notifications: boolean;
  /** Closing the window hides it and the app keeps checking from the tray. */
  keepInTray: boolean;
  account: Account | null;
  signedIn: boolean;
  ready: boolean;
  checking: boolean;
  lastCheck: number | null;
  nextCheck: number | null;
  notice: string | null;
}

export interface Progress {
  id: string;
  received: number;
  total: number | null;
  speed: number;
}

export interface AddReport {
  added: number;
  duplicates: number;
  invalid: string[];
}

/** Runs a backend command; shows the error to the user and resolves to `undefined` if it fails. */
export type Run = <T = void>(cmd: string, args?: Record<string, unknown>) => Promise<T | undefined>;

export type Say = (text: string, tone?: "info" | "success" | "error") => void;
