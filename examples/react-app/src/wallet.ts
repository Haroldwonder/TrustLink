import {
  StellarWalletsKit,
  WalletNetwork,
  FREIGHTER_ID,
  XBULL_ID,
  FreighterModule,
  xBullModule,
} from "@creit.tech/stellar-wallets-kit";
import { getNetworkDetails } from "@stellar/freighter-api";

export const SUPPORTED_WALLETS = [
  { id: FREIGHTER_ID, name: "Freighter", url: "https://freighter.app" },
  { id: XBULL_ID, name: "xBull", url: "https://xbull.app" },
] as const;

export type SupportedWalletId = typeof FREIGHTER_ID | typeof XBULL_ID;

export interface WalletState {
  connected: boolean;
  address: string | null;
}

const kit = new StellarWalletsKit({
  network: WalletNetwork.TESTNET,
  selectedWalletId: FREIGHTER_ID,
  modules: [new FreighterModule(), new xBullModule()],
});

export async function connectWallet(walletId: SupportedWalletId = FREIGHTER_ID): Promise<string> {
  kit.setWallet(walletId);
  const { address } = await kit.getAddress();
  localStorage.setItem("wallet_address", address);
  localStorage.setItem("wallet_id", walletId);
  return address;
}

export async function getWalletAddress(): Promise<string | null> {
  const stored = localStorage.getItem("wallet_address");
  const storedWalletId = localStorage.getItem("wallet_id") as SupportedWalletId | null;
  if (!stored || !storedWalletId) return null;
  try {
    kit.setWallet(storedWalletId);
    const { address } = await kit.getAddress();
    if (address !== stored) {
      localStorage.setItem("wallet_address", address);
    }
    return address;
  } catch {
    return null;
  }
}

export async function disconnectWallet(): Promise<void> {
  localStorage.removeItem("wallet_address");
  localStorage.removeItem("wallet_id");
}

export async function getConnectedNetwork(): Promise<string | null> {
  // Only Freighter exposes network details via its extension API; other wallets
  // trust the network configured in the kit (TESTNET).
  const storedWalletId = localStorage.getItem("wallet_id");
  if (storedWalletId !== FREIGHTER_ID) return null;
  try {
    const details = await getNetworkDetails();
    if (details.error) return null;
    return details.networkPassphrase ?? null;
  } catch {
    return null;
  }
}

export async function sign(xdr: string, network: string): Promise<string> {
  const { signedTxXdr } = await kit.signTransaction(xdr, { networkPassphrase: network });
  return signedTxXdr;
}
