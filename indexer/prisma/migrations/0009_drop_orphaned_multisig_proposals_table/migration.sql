-- Drop the orphaned snake_case table created by 0004_add_multisig_proposals_table.
-- The MultisigProposal model maps to the "MultisigProposal" table created in 0003.
DROP TABLE IF EXISTS "multisig_proposals";
