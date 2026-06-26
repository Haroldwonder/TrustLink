import { scValToNative } from "@stellar/stellar-sdk";
import { TrustLinkClient } from "../src/client";

describe("TrustLinkClient.getDelegation", () => {
  const CONTRACT_ID = "CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
  const DELEGATOR = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";
  const DELEGATE = "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";
  const CLAIM_TYPE = "KYC_PASSED";

  let client: TrustLinkClient;
  let simulateSpy: jest.SpyInstance;

  beforeEach(() => {
    client = new TrustLinkClient({ contractId: CONTRACT_ID, network: "testnet" });
    simulateSpy = jest.spyOn(client as any, "simulate");
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  test("calls the contract with address and string arguments", async () => {
    simulateSpy.mockResolvedValue(null);

    await client.getDelegation(DELEGATOR, DELEGATE, CLAIM_TYPE);

    expect(simulateSpy).toHaveBeenCalledWith(
      "get_delegation",
      expect.anything(),
      expect.anything(),
      expect.anything()
    );
    expect(simulateSpy.mock.calls[0][1].switch().name).toBe("scvAddress");
    expect(simulateSpy.mock.calls[0][2].switch().name).toBe("scvAddress");
    expect(scValToNative(simulateSpy.mock.calls[0][3])).toBe(CLAIM_TYPE);
  });

  test("returns null when no delegation exists", async () => {
    simulateSpy.mockResolvedValue(null);

    await expect(client.getDelegation(DELEGATOR, DELEGATE, CLAIM_TYPE)).resolves.toBeNull();
  });

  test("returns the decoded delegation struct", async () => {
    const delegation = {
      delegator: DELEGATOR,
      delegate: DELEGATE,
      claim_type: CLAIM_TYPE,
      expiration: 123n,
    };
    simulateSpy.mockResolvedValue(delegation);

    await expect(client.getDelegation(DELEGATOR, DELEGATE, CLAIM_TYPE)).resolves.toEqual(delegation);
  });
});
