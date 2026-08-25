"use client";

import { WalletRole } from "@/lib/governance/role-detection";
import { DashboardStats } from "@/components/governance/shared/DashboardStats";
import { RoleToggle } from "@/components/governance/shared/RoleToggle";
import { GovernanceActions } from "@/components/governance/shared/GovernanceActions";
import { ValidatorActionPanel } from "@/components/governance/validator/ValidatorActionPanel";
import { StakerActionPanel } from "@/components/governance/staker/StakerActionPanel";
import { VoteAccountsTable } from "@/components/governance/validator/VoteAccountsTable";
import { StakeAccountsTable } from "@/components/governance/staker/StakeAccountsTable";
import { SummaryStats } from "@/components/governance/staker/SummaryStats";
import { useWalletRole } from "@/hooks";

interface Props {
  userPubKey: string;
}

export function GovernanceDashboardLayout({ userPubKey }: Props) {
  const {
    walletRole,
    selectedView,
    setSelectedView,
    isLoading: isLoadingView,
  } = useWalletRole(userPubKey);

  const hasSelectedView = selectedView !== undefined;

  const showStats =
    (walletRole !== WalletRole.NONE && hasSelectedView) || isLoadingView;

  const showValidatorView =
    walletRole === WalletRole.VALIDATOR ||
    (walletRole === WalletRole.BOTH && selectedView === "validator");

  const showStakerView =
    walletRole === WalletRole.STAKER ||
    (walletRole === WalletRole.BOTH && selectedView === "staker");

  const canSwitchView = walletRole == WalletRole.BOTH;

  return (
    <>
      {/* Desktop Layout - hidden on mobile/tablet */}
      <div className="hidden lg:block">
        <div className="space-y-6">
          <div className="glass-card px-8 py-6 mb-6">
            <div className="flex items-center justify-between mb-6">
              <div>
                <h2 className="h2 text-foreground mb-2">
                  Governance Dashboard
                </h2>

                <p className="text-(--color-dao-color-gray) text-sm flex items-center">
                  You are viewing as a &nbsp;
                  {isLoadingView ? (
                    <span className="flex h-3 w-20 bg-white/10 animate-pulse rounded-full ml-2" />
                  ) : (
                    <span className="font-semibold text-primary">
                      {selectedView === "validator" ? "Validator" : "Staker"}
                    </span>
                  )}
                </p>
              </div>
              {walletRole !== WalletRole.NONE && selectedView && (
                <RoleToggle
                  currentView={selectedView}
                  onViewChange={setSelectedView}
                  canToggle={canSwitchView}
                />
              )}
            </div>

            {showStats && selectedView && (
              <DashboardStats
                currentView={selectedView}
                isLoading={isLoadingView}
              />
            )}
          </div>

          {showValidatorView && <ValidatorActionPanel />}

          {showStakerView && (
            <StakerActionPanel
              userPubKey={userPubKey}
              isLoading={isLoadingView}
            />
          )}
        </div>
      </div>

      {/* Mobile/Tablet Layout - hidden on desktop */}
      <div className="block lg:hidden">
        <div className="space-y-10 pb-10">
          <header className="space-y-6">
            <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
              <div>
                {isLoadingView ? (
                  <div className="h-7 w-30 my-1 bg-white/10 animate-pulse rounded-lg" />
                ) : (
                  <h1 className="text-3xl font-semibold text-foreground">
                    {selectedView === "validator" ? "Validator" : "Staker"}
                  </h1>
                )}
                {isLoadingView ? (
                  <div className="h-4 w-60 mt-1 bg-white/10 animate-pulse rounded-lg" />
                ) : (
                  <p className="text-sm text-white/60">
                    Oversee your {selectedView} operations and governance.
                  </p>
                )}
              </div>
              {walletRole !== WalletRole.NONE && selectedView && (
                <div className="sm:self-start">
                  <RoleToggle
                    currentView={selectedView}
                    onViewChange={setSelectedView}
                    canToggle={canSwitchView}
                  />
                </div>
              )}
            </div>
          </header>

          {showValidatorView && (
            <div className="space-y-8">
              <div className="space-y-3">
                <GovernanceActions
                  variant="validator"
                  title="Quick Actions"
                  description=""
                  wrapperClassName="space-y-4"
                  gridClassName="grid grid-cols-1 gap-y-5 sm:grid-cols-2 sm:gap-x-4 sm:gap-y-5"
                  isLoading={isLoadingView}
                />
              </div>

              {showStats && (
                <section className="space-y-4 pt-2 sm:pt-0">
                  <div>
                    <h3 className="h3 font-semibold text-foreground">
                      Governance Dashboard
                    </h3>
                  </div>
                  <div className="glass-card p-5">
                    <DashboardStats
                      currentView="validator"
                      isLoading={isLoadingView}
                    />
                  </div>
                </section>
              )}

              <VoteAccountsTable />
            </div>
          )}

          {showStakerView && (
            <div className="space-y-8">
              <GovernanceActions
                variant="staker"
                title="Quick Actions"
                description=""
                wrapperClassName="space-y-4"
                gridClassName="grid grid-cols-1 gap-y-5 sm:grid-cols-2 sm:gap-x-4 sm:gap-y-5"
                isLoading={isLoadingView}
              />

              {showStats && (
                <section className="space-y-4 pt-2 sm:pt-0">
                  <div>
                    <h3 className="h3 font-semibold">Governance Dashboard</h3>
                  </div>
                  <div className="glass-card p-5">
                    <DashboardStats
                      currentView="staker"
                      isLoading={isLoadingView}
                    />
                  </div>
                </section>
              )}

              <SummaryStats isLoading={isLoadingView} />
              <StakeAccountsTable userPubKey={userPubKey} />
            </div>
          )}
        </div>
      </div>
    </>
  );
}

// local vote account
// const YOUR_VOTE_ACCOUNT_PUBKEY = new PublicKey(
//   "GMJdVGehfUr327xwwCspjZ5aAL1vZ3KS3FV6ZasK38vQ"
// );

// const YOUR_VOTE_ACCOUNT_PUBKEY = new PublicKey(
//   "CsE3MdkQXYwEFuNsEkLajgqSEiLqN6aH7eBzhCdJecar"
// );

//  <div>
//    implementing stuff
//    <div>
//      <button onClick={createAndDelegateStakeAccount}>
//        create stake account
//      </button>
//    </div>
//  </div>;

// async function createAndDelegateStakeAccount() {
//   const connection = new Connection(endpoint, "confirmed");

//   const stakeAccount = Keypair.generate();
//   const rentExemption = await connection.getMinimumBalanceForRentExemption(
//     StakeProgram.space
//   );

//   // Amount to stake (1 SOL)
//   const amount = 10 * LAMPORTS_PER_SOL;

//   const lamportsToFund = Math.ceil(rentExemption + amount);

//   // 1️⃣ Create and fund the stake account
//   const createTx = StakeProgram.createAccount({
//     fromPubkey: userPubKey,
//     stakePubkey: stakeAccount.publicKey,
//     authorized: { staker: userPubKey, withdrawer: userPubKey },
//     lamports: lamportsToFund,
//   });

//   const sig1 = await sendTransaction(createTx, connection, {
//     signers: [stakeAccount],
//   });
//   await connection.confirmTransaction(sig1, "confirmed");

//   // 2️⃣ Choose a validator’s vote account (any testnet one)
//   // const { current } = await connection.getVoteAccounts();

//   // 3️⃣ Delegate the stake
//   const delegateTx = StakeProgram.delegate({
//     stakePubkey: stakeAccount.publicKey,
//     authorizedPubkey: userPubKey,
//     votePubkey: YOUR_VOTE_ACCOUNT_PUBKEY,
//   });

//   const sig2 = await sendTransaction(delegateTx, connection);
//   await connection.confirmTransaction(sig2, "confirmed");
//   console.log("Stake delegated to:", YOUR_VOTE_ACCOUNT_PUBKEY.toBase58());

//   // // 4️⃣ Send both in one transaction
//   // const tx = new Transaction().add(
//   //   ...createTx.instructions,
//   //   ...delegateTx.instructions
//   // );

//   // const signature = await sendTransaction(tx, connection, {
//   //   signers: [stakeAccount],
//   // });
//   // console.log(
//   //   "Stake account created & delegated:",
//   //   stakeAccount.publicKey.toBase58()
//   // );
//   // console.log("Transaction:", signature);
// }
