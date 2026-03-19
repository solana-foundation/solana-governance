import type { IdlAccounts, Program } from "@coral-xyz/anchor";

import { SvmgovProgram } from "./svmgov_program";

export type SvmgovProgramType = Program<SvmgovProgram>;

export type ProposalAccount = IdlAccounts<SvmgovProgram>["proposal"];
export type VoteAccount = IdlAccounts<SvmgovProgram>["vote"];
export type VoteOverrideAccount = IdlAccounts<SvmgovProgram>["voteOverride"];
export type SupportAccount = IdlAccounts<SvmgovProgram>["support"];
export type GlobalConfigAccount = IdlAccounts<SvmgovProgram>["globalConfig"];
