import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import IssuerDashboard from "../panels/IssuerDashboard";

// Mock the contract module
vi.mock("../contract", () => ({
  getIssuerStats: vi.fn(),
  getIssuerAttestations: vi.fn(),
  getExpiringAttestations: vi.fn(),
}));

import * as contract from "../contract";

const mockStats = { total_issued: 5, active: 3, revoked: 1, expired: 1 };

beforeEach(() => {
  vi.mocked(contract.getIssuerStats).mockResolvedValue(mockStats);
  vi.mocked(contract.getIssuerAttestations).mockResolvedValue([]);
  vi.mocked(contract.getExpiringAttestations).mockResolvedValue([]);
});

describe("IssuerDashboard", () => {
  it("shows loading state initially", () => {
    vi.mocked(contract.getIssuerStats).mockReturnValue(new Promise(() => {}));
    render(<IssuerDashboard address="GISSUER" />);
    expect(screen.getByText("Loading…")).toBeTruthy();
  });

  it("renders stats after successful fetch", async () => {
    render(<IssuerDashboard address="GISSUER" />);
    await waitFor(() => expect(screen.queryByText("Loading…")).toBeNull());
    expect(screen.getByText("5")).toBeTruthy(); // total_issued
    expect(screen.getByText("3")).toBeTruthy(); // active
  });

  it("shows error when getIssuerStats fails", async () => {
    vi.mocked(contract.getIssuerStats).mockRejectedValue(new Error("fetch failed"));
    render(<IssuerDashboard address="GISSUER" />);
    await waitFor(() => expect(screen.queryByText("Loading…")).toBeNull());
    expect(screen.getByText("fetch failed")).toBeTruthy();
  });

  it("uses useIssuerStats hook (getIssuerStats called with issuer address)", async () => {
    render(<IssuerDashboard address="GTEST" />);
    await waitFor(() => expect(contract.getIssuerStats).toHaveBeenCalledWith("GTEST"));
  });
});
