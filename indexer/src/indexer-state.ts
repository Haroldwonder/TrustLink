let lastLedger = 0;

export function getLastLedger(): number {
  return lastLedger;
}

export function setLastLedger(ledger: number): void {
  lastLedger = ledger;
}
