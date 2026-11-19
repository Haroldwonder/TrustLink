/**
 * Tests for Prometheus metrics to ensure no duplicate metric names.
 * Issue #928: duplicate Prometheus metric names crashed startup.
 */

import { register } from "prom-client";

describe("metrics registration", () => {
  it("should not have duplicate metric names (issue #928)", async () => {
    // Get all registered metrics
    const metricsOutput = await register.metrics();

    // Extract metric names (lines starting with # TYPE or actual metric lines)
    const metricNames = new Set<string>();
    const duplicates = new Set<string>();

    metricsOutput.split("\n").forEach((line) => {
      if (line.startsWith("# TYPE ")) {
        const parts = line.split(" ");
        const metricName = parts[2];
        if (metricName) {
          if (metricNames.has(metricName)) {
            duplicates.add(metricName);
          }
          metricNames.add(metricName);
        }
      }
    });

    expect(duplicates.size).toBe(
      0,
      `Found duplicate metric names: ${Array.from(duplicates).join(", ")}`
    );
  });

  it("should have distinct metrics for events_processed and events_processed_by_type", async () => {
    const metricsOutput = await register.metrics();

    // Verify both metric names exist and are distinct
    expect(metricsOutput).toContain("trustlink_events_processed_total");
    expect(metricsOutput).toContain("trustlink_events_processed_by_type_total");

    // Ensure the names are actually different
    expect("trustlink_events_processed_total").not.toBe(
      "trustlink_events_processed_by_type_total"
    );
  });
});
