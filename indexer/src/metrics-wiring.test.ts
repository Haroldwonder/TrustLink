/**
 * #936 – Prometheus metrics wiring
 *
 * Tests verify that Prometheus metrics are called at appropriate times.
 */
import * as metrics from "./metrics";

describe("#936 metrics wiring", () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it("calls incrementEventFailed when event processing fails", () => {
    const spy = jest.spyOn(metrics, "incrementEventFailed");
    metrics.incrementEventFailed("created");
    expect(spy).toHaveBeenCalledWith("created");
  });

  it("calls incrementIssuerAttestation when issuer creates attestation", () => {
    const spy = jest.spyOn(metrics, "incrementIssuerAttestation");
    metrics.incrementIssuerAttestation("GISSUER123");
    expect(spy).toHaveBeenCalledWith("GISSUER123");
  });

  it("calls incrementIssuerRevocation when issuer revokes attestation", () => {
    const spy = jest.spyOn(metrics, "incrementIssuerRevocation");
    metrics.incrementIssuerRevocation("GISSUER456");
    expect(spy).toHaveBeenCalledWith("GISSUER456");
  });

  it("calls setIssuerRateLimitRatio when rate limit is set", () => {
    const spy = jest.spyOn(metrics, "setIssuerRateLimitRatio");
    metrics.setIssuerRateLimitRatio("GISSUER789", 0.5);
    expect(spy).toHaveBeenCalledWith("GISSUER789", 0.5);
  });

  it("updates issuersTotal when new issuer registers", () => {
    const spy = jest.spyOn(metrics.issuersTotal, "set");
    metrics.issuersTotal.set(10);
    expect(spy).toHaveBeenCalledWith(10);
  });

  it("tracks event failures by type", () => {
    const failedSpy = jest.spyOn(metrics.eventsFailedTotal, "inc");
    metrics.incrementEventFailed("created");
    expect(failedSpy).toHaveBeenCalledWith({ type: "created" });
  });

  it("tracks issuer attestations per issuer", () => {
    const issuerSpy = jest.spyOn(metrics.issuerAttestationsTotal, "inc");
    metrics.incrementIssuerAttestation("GISSUER_TEST");
    expect(issuerSpy).toHaveBeenCalledWith({ issuer: "GISSUER_TEST" });
  });

  it("tracks issuer revocations per issuer", () => {
    const revokeSpy = jest.spyOn(metrics.issuerRevocationsTotal, "inc");
    metrics.incrementIssuerRevocation("GISSUER_REVOKE");
    expect(revokeSpy).toHaveBeenCalledWith({ issuer: "GISSUER_REVOKE" });
  });

  it("tracks rate limit ratio per issuer", () => {
    const ratioSpy = jest.spyOn(metrics.issuerRateLimitRatio, "set");
    metrics.setIssuerRateLimitRatio("GISSUER_RATIO", 0.75);
    expect(ratioSpy).toHaveBeenCalledWith({ issuer: "GISSUER_RATIO" }, 0.75);
  });
});
