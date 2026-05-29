import { useState, useEffect } from "react";
import { connectWallet, getWalletAddress } from "./wallet";
import { ErrorBoundary } from "./ErrorBoundary";
import AdminPanel from "./panels/AdminPanel";
import IssuerPanel from "./panels/IssuerPanel";
import UserPanel from "./panels/UserPanel";
import VerifierPanel from "./panels/VerifierPanel";
import AttestationRequestPanel from "./panels/AttestationRequestPanel";
import MultiSigPanel from "./panels/MultiSigPanel";

type Tab = "admin" | "issuer" | "user" | "verifier" | "requests" | "multisig";

export default function App() {
  const [address, setAddress] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("user");
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [darkMode, setDarkMode] = useState<boolean>(() => {
    const stored = localStorage.getItem("trustlink-theme");
    return stored ? stored === "dark" : true;
  });

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", darkMode ? "dark" : "light");
    localStorage.setItem("trustlink-theme", darkMode ? "dark" : "light");
  }, [darkMode]);

  // Auto-reconnect if Freighter is already authorised
  useEffect(() => {
    getWalletAddress().then((addr) => { if (addr) setAddress(addr); });
  }, []);

  async function handleConnect() {
    setConnecting(true);
    setError(null);
    try {
      const addr = await connectWallet();
      setAddress(addr);
    } catch (e: unknown) {
      setError((e as Error).message);
    } finally {
      setConnecting(false);
    }
  }

  async function handleDisconnect() {
    await disconnectWallet();
    setAddress(null);
    setTab("user");
    setError(null);
  }

  const short = address ? `${address.slice(0, 6)}…${address.slice(-4)}` : "";

  if (!address) {
    return (
      <div className="connect-screen">
        <h2>TrustLink dApp</h2>
        <p>Connect your Freighter wallet to interact with the TrustLink attestation contract on Stellar testnet.</p>
        {error && <div className="alert alert-error">{error}</div>}
        <button className="btn btn-primary" style={{ fontSize: "1rem", padding: "0.75rem 2rem" }} disabled={connecting} onClick={handleConnect}>
          {connecting ? "Connecting…" : "Connect Freighter"}
        </button>
        <p style={{ fontSize: "0.75rem", color: "#475569" }}>
          Don't have Freighter?{" "}
          <a href="https://freighter.app" target="_blank" rel="noreferrer" style={{ color: "#7c6af7" }}>
            freighter.app
          </a>
        </p>
      </div>
    );
  }

  const TABS: { id: Tab; label: string }[] = [
    { id: "user", label: "My Attestations" },
    { id: "requests", label: "Requests" },
    { id: "multisig", label: "Multi-Sig" },
    { id: "issuer", label: "Issuer" },
    { id: "verifier", label: "Verifier" },
    { id: "admin", label: "Admin" },
  ];

  return (
    <>
      <header className="header">
        <h1>TrustLink</h1>
        <div className="wallet-info">
          <button className="btn btn-outline theme-toggle" onClick={() => setDarkMode((d) => !d)} aria-label="Toggle dark mode">
            {darkMode ? "☀️" : "🌙"}
          </button>
          <span className="addr">{short}</span>
          <button className="btn btn-outline" style={{ fontSize: "0.8rem", padding: "0.3rem 0.75rem" }} onClick={handleDisconnect}>
            Disconnect
          </button>
        </div>
      </header>

      <nav className="tabs">
        {TABS.map((t) => (
          <button key={t.id} className={`tab ${tab === t.id ? "active" : ""}`} onClick={() => setTab(t.id)}>
            {t.label}
          </button>
        ))}
      </nav>

      {tab === "user" && <ErrorBoundary><UserPanel address={address} /></ErrorBoundary>}
      {tab === "requests" && <ErrorBoundary><AttestationRequestPanel address={address} /></ErrorBoundary>}
      {tab === "multisig" && <ErrorBoundary><MultiSigPanel address={address} /></ErrorBoundary>}
      {tab === "issuer" && <ErrorBoundary><IssuerPanel address={address} /></ErrorBoundary>}
      {tab === "verifier" && <ErrorBoundary><VerifierPanel /></ErrorBoundary>}
      {tab === "admin" && <ErrorBoundary><AdminPanel address={address} /></ErrorBoundary>}
    </>
  );
}
