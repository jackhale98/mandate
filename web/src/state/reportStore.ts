// "Report a problem" (bug bundles for testers): whether the dialog is open,
// and what a report carries beyond the run itself: the last few errors the
// app caught, each with the command that was being run when it happened.
// Nothing here leaves the device unless the tester saves and sends a report.
import { create } from "zustand";
import type { CommandDto } from "../engine/protocol/CommandDto";

/** How many recent errors a report keeps. */
export const MAX_REPORT_ERRORS = 8;

export interface CaughtError {
  /** ISO wall-clock time it was caught (metadata only). */
  at: string;
  message: string;
  /** The last command sent to the engine before it, if any. */
  command: CommandDto | null;
}

/** Where the dialog was opened from (the error banner preselects nothing else; kept for the report). */
export type ReportOrigin = "menu" | "error" | "title";

interface ReportState {
  open: boolean;
  origin: ReportOrigin;
  errors: CaughtError[];
  openReport(origin: ReportOrigin): void;
  closeReport(): void;
}

let lastCommand: CommandDto | null = null;

/** Remember the command being sent to the engine (so an error can say what was being done). */
export function noteCommand(cmd: CommandDto) {
  lastCommand = cmd;
}

/** Keep an error for the next report (the oldest goes once there are more than `MAX_REPORT_ERRORS`). */
export function recordError(message: string, now = new Date()) {
  const e: CaughtError = { at: now.toISOString(), message, command: lastCommand };
  useReport.setState((s) => ({ errors: [...s.errors, e].slice(-MAX_REPORT_ERRORS) }));
}

export const useReport = create<ReportState>((set) => ({
  open: false,
  origin: "menu",
  errors: [],
  openReport(origin) {
    set({ open: true, origin });
  },
  closeReport() {
    set({ open: false });
  },
}));
