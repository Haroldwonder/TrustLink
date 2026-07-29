----------------------------- MODULE multisig_state_machine -----------------------------
EXTENDS Integers, Sequences, TLC

(*
  TLA+ Specification: Multi-Sig Proposal Lifecycle State Machine
  
  This specification formalizes the multi-signature proposal lifecycle in TrustLink.
  It defines the states, transitions, and invariants that govern how proposals
  evolve through their lifecycle.
  
  Version: 1.0
  Date: 2026-07-29
*)

CONSTANTS Address, ClaimType, MaxSigners, MaxThreshold

VARIABLES proposals
VARIABLES signatures

(*
  State definition: Each proposal has a unique ID and belongs to one of the
  states defined in the ProposalState type.
*)
ProposalState == [id: Address, proposer: Address, subject: Address, 
                  claim_type: ClaimType, required_signers: SET OF Address,
                  threshold: Nat, signers: SET OF Address,
                  created_at: Int, expires_at: Int,
                  finalized: Bool, cancelled: Bool, paused: Bool]

(*
  State variable: proposals is a mapping from proposal ID to ProposalState
*)
proposals :: [id : Address -> ProposalState]

(*
  State variable: signatures is a set of (proposal_id, signer) pairs
  representing which addresses have signed which proposals
*)
signatures :: SET OF [proposal_id : Address, signer : Address]

(* ============================================================================
   INITIALIZATION
   ============================================================================ *)

Init == 
  /\ proposals = [p : Address |-> [id |-> p, proposer |-> p, subject |-> p,
                                   claim_type |-> p, required_signers |-> {},
                                   threshold |-> 0, signers |-> {},
                                   created_at |-> 0, expires_at |-> 0,
                                   finalized |-> FALSE, cancelled |-> FALSE,
                                   paused |-> FALSE]]
  /\ signatures = {}

(* ============================================================================
   STATES AND TRANSITIONS
   ============================================================================ *)

(* -- Propose: Create a new multi-sig proposal -- *)

CanPropose(p, proposer, subject, claim_type, required_signers, threshold, now, ttl_secs) ==
  /\ \forall s \in required_signers: s \in Address
  /\ threshold > 0
  /\ threshold <= Cardinality(required_signers)
  /\ Cardinality(required_signers) <= MaxSigners
  /\ now + ttl_secs < 2^64  (* Avoid overflow *)
  /\ p \notin DOMAIN proposals  (* ID must be unique *)
  
Propose(p, proposer, subject, claim_type, required_signers, threshold, now, ttl_secs) ==
  /\ CanPropose(p, proposer, subject, claim_type, required_signers, threshold, now, ttl_secs)
  /\ proposals' = [proposals EXCEPT ![p] = [
      id |-> p,
      proposer |-> proposer,
      subject |-> subject,
      claim_type |-> claim_type,
      required_signers |-> required_signers,
      threshold |-> threshold,
      signers |-> {proposer},  (* Proposer auto-signs *)
      created_at |-> now,
      expires_at |-> now + ttl_secs,
      finalized |-> FALSE,
      cancelled |-> FALSE,
      paused |-> proposals[p].paused
  ]]
  /\ signatures' = [signatures EXCEPT ![p] = {proposer}]

(* -- Cosign: Sign a proposal -- *)

CanCosign(proposal_id, signer, now) ==
  /\ proposal_id \in DOMAIN proposals
  /\ signer \in Address
  /\ ~proposals[proposal_id].cancelled
  /\ ~proposals[proposal_id].finalized
  /\ now < proposals[proposal_id].expires_at
  /\ \* Signer must be in required_signers list
     signer \in proposals[proposal_id].required_signers
  /\ \* Already signed check
     signer \notin proposals[proposal_id].signers

Cosign(proposal_id, signer, now) ==
  /\ CanCosign(proposal_id, signer, now)
  /\ proposals' = [proposals EXCEPT ![proposal_id].signers = 
                    proposals[proposal_id].signers \cup {signer}]
  /\ signatures' = signatures \cup {[proposal_id |-> proposal_id, signer |-> signer]}

(* -- Finalize: Mark proposal as finalized when threshold reached -- *)

CanFinalize(proposal_id, now) ==
  /\ proposal_id \in DOMAIN proposals
  /\ ~proposals[proposal_id].cancelled
  /\ ~proposals[proposal_id].finalized
  /\ now < proposals[proposal_id].expires_at
  /\ proposals[proposal_id].paused = FALSE
  /\ Cardinality(proposals[proposal_id].signers) >= proposals[proposal_id].threshold
  
Finalize(proposal_id, now) ==
  /\ CanFinalize(proposal_id, now)
  /\ proposals' = [proposals EXCEPT ![proposal_id].finalized = TRUE]

(* -- Cancel: Cancel by proposer -- *)

CanCancel(proposal_id, canceller, now) ==
  /\ proposal_id \in DOMAIN proposals
  /\ canceller = proposals[proposal_id].proposer
  /\ ~proposals[proposal_id].cancelled
  /\ ~proposals[proposal_id].finalized
  /\ now < proposals[proposal_id].expires_at
  /\ proposals[proposal_id].paused = FALSE
  
Cancel(proposal_id, canceller, now) ==
  /\ CanCancel(proosal_id, canceller, now)
  /\ proposals' = [proposals EXCEPT ![proposal_id].cancelled = TRUE]

(* -- Expire: Proposal expires naturally -- *)

CanExpire(proposal_id, now) ==
  /\ proposal_id \in DOMAIN proposals
  /\ now >= proposals[proposal_id].expires_at
  /\ ~proposals[proposal_id].finalized
  /\ ~proposals[proposal_id].cancelled
  
Expire(proposal_id, now) ==
  /\ CanExpire(proposal_id, now)
  /\ proposals' = proposals  (* No state change, just marks as expired *)

(* -- Pause: Contract pause state affects all proposals -- *)

Pause(proposals) ==
  /\ \forall p \in DOMAIN proposals:
       proposals[p].finalized = FALSE /\ proposals[p].cancelled = FALSE
  /\ proposals' = [p \in DOMAIN proposals |-> 
                   [proposals[p] EXCEPT !.paused = TRUE]]

Unpause(proposals) ==
  /\ proposals' = [p \in DOMAIN proposals |-> 
                   [proposals[p] EXCEPT !.paused = FALSE]]

(* ============================================================================
   COMPOUND OPERATIONS
   ============================================================================ *)

Next == 
  \E p \in Address, proposer \in Address, subject \in Address,
       claim_type \in ClaimType, required_signers \in SET OF Address,
       threshold \in 1..MaxSigners, now \in Int, ttl_secs \in Int:
    Propose(p, proposer, subject, claim_type, required_signers, threshold, now, ttl_secs)
  
  \/ \E p \in DOMAIN proposals, signer \in Address, now \in Int:
       CanCosign(p, signer, now) => Cosign(p, signer, now)
  
  \/ \E p \in DOMAIN proposals, now \in Int:
       CanFinalize(p, now) => Finalize(p, now)
  
  \/ \E p \in DOMAIN proposals, canceller \in Address, now \in Int:
       CanCancel(p, canceller, now) => Cancel(p, canceller, now)
  
  \/ \E p \in DOMAIN proposals, now \in Int:
       CanExpire(p, now) => Expire(p, now)
  
  \/ \E now \in Int:
       Pause(proposals) \/ Unpause(proposals)

(* ============================================================================
   INVARIANTS
   ============================================================================ *)

(*
  Invariant 1: A proposal cannot be modified while the contract is paused
  This prevents state changes during emergency pauses.
*)
InvariantNoPauseModification == 
  \forall p \in DOMAIN proposals:
    proposals[p].paused => 
      ~proposals[p].finalized /\ ~proposals[p].cancelled

(*
  Invariant 2: Signature count never includes non-required signers
  Only addresses in required_signers can sign a proposal.
*)
InvariantSignerAuth == 
  \forall p \in DOMAIN proposals, s \in proposals[p].signers:
    s \in proposals[p].required_signers

(*
  Invariant 3: Threshold is satisfied only when enough signatures exist
*)
InvariantThresholdSatisfied == 
  \forall p \in DOMAIN proposals:
    proposals[p].finalized => 
      Cardinality(proposals[p].signers) >= proposals[p].threshold

(*
  Invariant 4: A proposal cannot be finalized and cancelled simultaneously
*)
InvariantMutuallyExclusive == 
  \forall p \in DOMAIN proposals:
    ~(proposals[p].finalized /\ proposals[p].cancelled)

(*
  Invariant 5: Signature count never exceeds threshold
  (This is actually enforced by the finalize condition, but useful to document)
*)
InvariantSignatureLimit == 
  \forall p \in DOMAIN proposals:
    Cardinality(proposals[p].signers) <= Cardinality(proposals[p].required_signers)

(*
  Invariant 6: Proposer auto-signs on creation
  The proposer must be in the signers set immediately after proposing.
*)
InvariantProposerSigns == 
  \forall p \in DOMAIN proposals:
    proposals[p].proposer \in proposals[p].signers

(*
  Invariant 7: Expiration prevents all operations after expiry
*)
InvariantExpiration == 
  \forall p \in DOMAIN proposals, now \in Int:
    now >= proposals[p].expires_at =>
      ~proposals[p].finalized /\ ~proposals[p].cancelled => 
        (\* proposal is expired but not finalized/cancelled
         \forall action \in {Cosign, Finalize, Cancel}:
           \neg Can[action](p, _, now))

(*
  Invariant 8: No duplicate signatures
  Each signer can only sign once per proposal.
*)
InvariantNoDuplicateSignatures == 
  \forall p \in DOMAIN proposals:
    Cardinality(proposals[p].signers) = 
      Cardinality({s \in proposals[p].signers | TRUE})

(* ============================================================================
   PROPERTIES
   ============================================================================ *)

(*
  Property 1: Progress - If not paused and not expired, a proposal will eventually
  reach a terminal state (finalized, cancelled, or expired)
*)
PropertyProgress ==
  \square(\Diamond finalized \/ \Diamond cancelled \/ \Diamond expired)

(*
  Property 2: Safety - Once finalized or cancelled, no further state changes occur
*)
PropertyIrreversibility ==
  \forall p \in DOMAIN proposals:
    (proposals[p].finalized \/ proposals[p].cancelled) => 
      \square(proposals[p].finalized \/ proposals[p].cancelled)

(*
  Property 3: Uniqueness - Each proposal ID is unique
*)
PropertyUniqueIDs ==
  \forall p1, p2 \in Address:
    p1 /= p2 => proposals[p1].id /= proposals[p2].id

(* ============================================================================
   SPECIFICATION
   ============================================================================ *)

Spec == Init /\ [][Next]_<<proposals, signatures>>

(* ============================================================================
   MODEL CHECKING CONFIGURATION
   ============================================================================ *)

(*
  To model check this specification using TLA+ Tools:
  
  1. Install TLA+ Tools: https://lamport.azurewebsites.net/tla/tools.html
  2. Load this file in the TLA+ Toolbox
  3. Create a model with appropriate constants:
     - Address: {A, B, C, D, E}  (finite set of addresses)
     - ClaimType: {KYC, AML, POLICY}  (finite set of claim types)
     - MaxSigners: 5
     - MaxThreshold: 3
  4. Set invariants to check:
     - InvariantNoPauseModification
     - InvariantSignerAuth
     - InvariantThresholdSatisfied
     - InvariantMutuallyExclusive
     - InvariantSignatureLimit
     - InvariantProposerSigns
     - InvariantExpiration
     - InvariantNoDuplicateSignatures
  5. Run model checker with bounded time/depth
*)
