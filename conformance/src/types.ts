export type ConformanceObservation =
  | { step: string; kind: "boolean"; value: boolean }
  | { step: string; kind: "ids"; value: string[] }
  | { step: string; kind: "error"; code: number; name: string };

export interface ConformanceContext {
  contractId: string;
  rpcUrl: string;
  networkPassphrase: string;
  subject: string;
  issuer: string;
}
