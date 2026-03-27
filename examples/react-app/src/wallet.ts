import {
  isConnected,
  getAddress,
  signTransaction,
} from "@stellar/freighter-api";

export interface WalletState {
  connected: boolean;
  address: string | null;
}

export async function connectWallet(): Promise<string> {
  const connected = await isConnected();
  if (!connected) {
    throw new Error("Freighter wallet not found. Please install the Freighter extension.");
  }
  const result = await getAddress();
  if (result.error) throw new Error(result.error.message);
  return result.address;
}

export async function getWalletAddress(): Promise<string | null> {
  try {
    const connected = await isConnected();
    if (!connected) return null;
    const result = await getAddress();
    if (result.error) return null;
    return result.address;
  } catch {
    return null;
  }
}

export async function sign(xdr: string, network: string): Promise<string> {
  const result = await signTransaction(xdr, { networkPassphrase: network });
  if (result.error) throw new Error(result.error.message);
  return result.signedTxXdr;
}
