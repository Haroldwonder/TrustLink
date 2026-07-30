import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import i18n from "i18next";
import { Networks } from "@stellar/stellar-sdk";
import {
  connectWallet,
  getWalletAddress,
  getConnectedNetwork,
  disconnectWallet,
  SUPPORTED_WALLETS,
  type SupportedWalletId,
} from "./wallet";
import { ErrorBoundary } from "./ErrorBoundary";
import AdminPanel from "./panels/AdminPanel";
import IssuerPanel from "./panels/IssuerPanel";
import UserPanel from "./panels/UserPanel";
import VerifierPanel from "./panels/VerifierPanel";
import AttestationRequestPanel from "./panels/AttestationRequestPanel";
import MultiSigPanel from "./panels/MultiSigPanel";
import CouncilPanel from "./panels/CouncilPanel";
import DelegationPanel from "./panels/DelegationPanel";
import WhitelistPanel from "./panels/WhitelistPanel";
import { useAttestationSubscription } from "./hooks/useAttestationSubscription";
import { useToasts, ToastContainer } from "./Toast";

type Tab = "admin" | "issuer" | "user" | "verifier" | "requests" | "multisig" | "council" | "delegation" | "whitelist";

const SUPPORTED_LANGS = ["en", "es"] as const;

export default function App() {
  const { t } = useTranslation();
  const [address, setAddress] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("user");
  const [connecting, setConnecting] = useState<SupportedWalletId | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [networkMismatch, setNetworkMismatch] = useState(false);
  const [darkMode, setDarkMode] = useState<boolean>(() => {
    const stored = localStorage.getItem("trustlink-theme");
    return stored ? stored === "dark" : true;
  });
  const [lang, setLang] = useState<string>(() => i18n.language ?? "en");

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", darkMode ? "dark" : "light");
    localStorage.setItem("trustlink-theme", darkMode ? "dark" : "light");
  }, [darkMode]);

  useEffect(() => {
    getWalletAddress().then((addr) => { if (addr) setAddress(addr); });
  }, []);

  useEffect(() => {
    if (!address) { setNetworkMismatch(false); return; }
    getConnectedNetwork().then((passphrase) => {
      setNetworkMismatch(passphrase != null && passphrase !== Networks.TESTNET);
    });
  }, [address]);

  async function handleConnect(walletId: SupportedWalletId) {
    setConnecting(walletId);
    setError(null);
    try {
      const addr = await connectWallet(walletId);
      setAddress(addr);
      const passphrase = await getConnectedNetwork();
      setNetworkMismatch(passphrase != null && passphrase !== Networks.TESTNET);
    } catch (e: unknown) {
      setError((e as Error).message);
    } finally {
      setConnecting(null);
    }
  }

  async function handleDisconnect() {
    await disconnectWallet();
    setAddress(null);
    setTab("user");
    setError(null);
  }

  function cycleLang() {
    const idx = SUPPORTED_LANGS.indexOf(lang as typeof SUPPORTED_LANGS[number]);
    const next = SUPPORTED_LANGS[(idx + 1) % SUPPORTED_LANGS.length];
    i18n.changeLanguage(next);
    setLang(next);
  }

  const { toasts, push: pushToast, dismiss: dismissToast } = useToasts();
  useAttestationSubscription(address, pushToast);

  const short = address ? `${address.slice(0, 6)}…${address.slice(-4)}` : "";

  if (!address) {
    return (
      <div className="connect-screen">
        <h2>{t("app.dapp_title")}</h2>
        <p>{t("app.connect_prompt")}</p>
        {error && <div className="alert alert-error">{error}</div>}
        <p style={{ fontSize: "0.85rem", color: "#94a3b8", marginBottom: "0.75rem" }}>
          {t("wallet.choose")}
        </p>
        <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", justifyContent: "center" }}>
          {SUPPORTED_WALLETS.map(({ id, name }) => (
            <button
              key={id}
              className="btn btn-primary"
              style={{ fontSize: "1rem", padding: "0.75rem 2rem", minWidth: "160px" }}
              disabled={connecting !== null}
              onClick={() => handleConnect(id as SupportedWalletId)}
            >
              {connecting === id ? t("app.connecting") : t("app.connect_btn", { name })}
            </button>
          ))}
        </div>
        <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", justifyContent: "center" }}>
          {SUPPORTED_WALLETS.map(({ id, name, url }) => (
            <p key={id} style={{ fontSize: "0.75rem", color: "#475569" }}>
              {t(`wallet.${id === "freighter" ? "freighter" : "xbull"}`)}{" "}
              <a href={url} target="_blank" rel="noreferrer" style={{ color: "#7c6af7" }}>
                {url.replace("https://", "")}
              </a>
            </p>
          ))}
        </div>
      </div>
    );
  }

  const TABS: { id: Tab; label: string }[] = [
    { id: "user", label: t("tabs.user") },
    { id: "requests", label: t("tabs.requests") },
    { id: "multisig", label: t("tabs.multisig") },
    { id: "delegation", label: t("tabs.delegation") },
    { id: "whitelist", label: t("tabs.whitelist") },
    { id: "issuer", label: t("tabs.issuer") },
    { id: "verifier", label: t("tabs.verifier") },
    { id: "admin", label: t("tabs.admin") },
    { id: "council", label: t("tabs.council") },
  ];

  return (
    <>
      <header className="header">
        <h1>TrustLink</h1>
        <div className="wallet-info">
          <button
            className="btn btn-outline theme-toggle"
            onClick={cycleLang}
            aria-label="Switch language"
            style={{ fontSize: "0.75rem", padding: "0.3rem 0.6rem" }}
          >
            {lang.toUpperCase()}
          </button>
          <button className="btn btn-outline theme-toggle" onClick={() => setDarkMode((d) => !d)} aria-label={t("app.toggle_theme")}>
            {darkMode ? "☀️" : "🌙"}
          </button>
          <span className="addr">{short}</span>
          <button className="btn btn-outline" style={{ fontSize: "0.8rem", padding: "0.3rem 0.75rem" }} onClick={handleDisconnect}>
            {t("app.disconnect")}
          </button>
        </div>
      </header>

      <nav className="tabs">
        {TABS.map((tabItem) => (
          <button key={tabItem.id} className={`tab ${tab === tabItem.id ? "active" : ""}`} onClick={() => setTab(tabItem.id)}>
            {tabItem.label}
          </button>
        ))}
      </nav>

      {networkMismatch && (
        <div
          className="alert alert-error"
          style={{ margin: "1rem", borderRadius: "0.5rem", padding: "1rem 1.25rem", fontSize: "0.9rem" }}
        >
          <strong>{t("app.wrong_network")}</strong> {t("app.wrong_network_msg")}
        </div>
      )}

      {tab === "user" && <ErrorBoundary><UserPanel address={address} /></ErrorBoundary>}
      {tab === "requests" && <ErrorBoundary><AttestationRequestPanel address={address} /></ErrorBoundary>}
      {tab === "multisig" && <ErrorBoundary><MultiSigPanel address={address} /></ErrorBoundary>}
      {tab === "delegation" && <ErrorBoundary><DelegationPanel address={address} /></ErrorBoundary>}
      {tab === "whitelist" && <ErrorBoundary><WhitelistPanel address={address} /></ErrorBoundary>}
      {tab === "issuer" && <ErrorBoundary><IssuerPanel address={address} /></ErrorBoundary>}
      {tab === "verifier" && <ErrorBoundary><VerifierPanel /></ErrorBoundary>}
      {tab === "admin" && <ErrorBoundary><AdminPanel address={address} /></ErrorBoundary>}
      {tab === "council" && <ErrorBoundary><CouncilPanel address={address} /></ErrorBoundary>}
      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </>
  );
}
