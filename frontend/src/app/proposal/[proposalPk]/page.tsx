import { Suspense } from "react";
import { ProposalDetailClientPage } from "../ProposalDetailClientPage";

export default function ProposalPage() {
  return (
    <Suspense fallback={null /* TODO: or a skeleton */}>
      <ProposalDetailClientPage />
    </Suspense>
  );
}
