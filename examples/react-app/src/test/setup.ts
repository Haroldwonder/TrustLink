import "@testing-library/jest-dom";
import { configureAxe } from "vitest-axe";
import "vitest-axe/extend-expect";

// Configure axe for WCAG 2.1 AA compliance checks across all tests
configureAxe({
  rules: {
    // Enforce WCAG 2.1 AA rules
    "color-contrast": { enabled: true },
    "label": { enabled: true },
    "button-name": { enabled: true },
    "region": { enabled: true },
    "heading-order": { enabled: true },
    "landmark-one-main": { enabled: true },
  },
});
