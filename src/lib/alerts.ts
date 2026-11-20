// An alert as a sentence. Names come from the server, so they are cleaned.

import { clean, percent } from "./text";
import type { Alert } from "./types";

export function describe(alert: Alert): string {
  switch (alert.kind) {
    case "unit_failed":
      return `${clean(alert.unit)} failed`;
    case "disk_full":
      return `${clean(alert.mount)} is ${percent(alert.percent)} full`;
    case "memory_low":
      return `Only ${percent(alert.percent)} of memory is free`;
    case "unreachable":
      return `Not answering: ${clean(alert.error)}`;
    case "container_failed":
      return `Container ${clean(alert.container)}: ${clean(alert.status)}`;
    case "app_failed":
      return `PM2 app ${clean(alert.app)} errored`;
  }
}
