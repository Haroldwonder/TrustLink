import { render, screen, waitFor } from "@testing-library/react";
import { vi, describe, it, expect, beforeEach, afterEach } from "vitest";
import AdminPanel from "../panels/AdminPanel";
import * as contract from "../contract";

vi.mock("../contract", () => ({
  getGlobalStats: vi.fn(),
  registerIssuer: vi.fn(),
  removeIssuer: vi.fn(),
  isIssuer: vi.fn(),
}));

const mockStats = {
  total_attestations: 42n,
  total_revocations: 5n,
  total_issuers: 3n,
};

const ADDRESS = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";

describe("AdminPanel", () => {
  beforeEach(() => {
    vi.mocked(contract.getGlobalStats).mockResolvedValue(mockStats);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders the panel heading", () => {
    render(<AdminPanel address={ADDRESS} />);
    expect(screen.getByText("Admin Panel")).toBeInTheDocument();
  });

  it("shows loading state initially", () => {
    render(<AdminPanel address={ADDRESS} />);
    expect(screen.getByText("Loading…")).toBeInTheDocument();
  });

  it("displays global stats after loading", async () => {
    render(<AdminPanel address={ADDRESS} />);
    await waitFor(() => expect(screen.queryByText("Loading…")).not.toBeInTheDocument());
    expect(screen.getByText("42")).toBeInTheDocument();
    expect(screen.getByText("5")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
  });

  it("shows error when stats fetch fails", async () => {
    vi.mocked(contract.getGlobalStats).mockRejectedValue(new Error("rpc error"));
    render(<AdminPanel address={ADDRESS} />);
    await waitFor(() => expect(screen.getByText("rpc error")).toBeInTheDocument());
  });

  it("renders register and remove buttons", async () => {
    render(<AdminPanel address={ADDRESS} />);
    await waitFor(() => expect(screen.queryByText("Loading…")).not.toBeInTheDocument());
    expect(screen.getByText("Register")).toBeInTheDocument();
    expect(screen.getByText("Remove")).toBeInTheDocument();
  });

  it("renders check issuer button", async () => {
    render(<AdminPanel address={ADDRESS} />);
    await waitFor(() => expect(screen.queryByText("Loading…")).not.toBeInTheDocument());
    expect(screen.getByText("Check")).toBeInTheDocument();
  });
});
