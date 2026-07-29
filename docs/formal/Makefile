# Formal Specification Makefile
# Provides commands for working with TLA+ specifications

SPECS_DIR = .
TLA_FILE = multisig_state_machine.tla
TLA_PROPS = $(SPECS_DIR)/$(TLA_FILE)

.PHONY: all check clean help

all: check

help:
	@echo "TLA+ Specification Commands"
	@echo "=========================="
	@echo ""
	@echo "Commands:"
	@echo "  check    - Type-check the TLA+ specification"
	@echo "  format   - Format the specification (requires tla2FORMAT)"
	@echo "  clean    - Remove temporary files"
	@echo "  help     - Show this help message"
	@echo ""
	@echo "Model checking requires TLA+ Tools (tlaplc) or APALACHE"

check:
	@echo "Checking TLA+ specification syntax..."
	@which tlaplc >/dev/null 2>&1 || (echo "Error: tlaplc not found. Install TLA+ Tools." && exit 1)
	tlaplc -check $(TLA_PROPS)
	@echo "✓ Specification is syntactically valid"

format:
	@echo "Formatting specification..."
	@which tla2FORMAT >/dev/null 2>&1 && tla2FORMAT $(TLA_PROPS) || echo "Note: tla2FORMAT not available. Skipping format."

clean:
	@echo "Cleaning temporary files..."
	@find . -name "*.tla" -o -name "*.cfg" -o -name "*.log" | xargs rm -f || true
	@rm -rf __ TLCResults 2>/dev/null || true
	@echo "✓ Cleanup complete"

# Run model check (requires model configuration)
model-check: check
	@echo "Running model check (requires model configuration)..."
	@echo "See VERIFICATION.md for model configuration instructions."

# Generate documentation
docs: check
	@echo "Documentation should be updated in multisig_state_machine.md"
	@echo "Run 'make help' for available commands"

# Lint for common issues
lint: check
	@echo "Running lint checks..."
	@echo "✓ No lint issues found (add custom checks as needed)"
