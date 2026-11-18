import { jest } from "@jest/globals";
import { TrustLinkClient } from "../src/client.js";
import { InvalidAddressError } from "../src/validation.js";

describe("TrustLinkClient.getIssuerClaimTypes", () => {
  const CONTRACT_ID = "CAAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQC526";
  const ISSUER = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

  let client: TrustLinkClient;

  beforeEach(() => {
    client = new TrustLinkClient({ contractId: CONTRACT_ID, network: "testnet" });
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  test("validates issuer address format", async () => {
    await expect(client.getIssuerClaimTypes("invalid_address")).rejects.toThrow(
      InvalidAddressError
    );
  });

  test("calls contract get_issuer_claim_types and returns array of claim types", async () => {
    const simulateSpy = jest
      .spyOn(client as any, "simulate")
      .mockResolvedValue(["KYC_PASSED", "ACCREDITED_INVESTOR"]);

    const result = await client.getIssuerClaimTypes(ISSUER);

    expect(simulateSpy).toHaveBeenCalledWith(
      "get_issuer_claim_types",
      expect.anything()
    );
    expect(result).toEqual(["KYC_PASSED", "ACCREDITED_INVESTOR"]);
  });
});
