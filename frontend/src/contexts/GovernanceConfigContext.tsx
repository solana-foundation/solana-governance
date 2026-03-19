"use client";

import {
  createContext,
  useContext,
  type ReactNode,
} from "react";
import type { UseQueryResult } from "@tanstack/react-query";
import type { GovernanceConfigDto } from "@/lib/getGovernanceConfig";
import { useGovernanceConfig } from "@/hooks/useGovernanceConfig";

export type GovernanceConfigContextValue = UseQueryResult<GovernanceConfigDto>;

const GovernanceConfigContext = createContext<
  GovernanceConfigContextValue | undefined
>(undefined);

export function GovernanceConfigProvider({ children }: { children: ReactNode }) {
  const query = useGovernanceConfig();
  return (
    <GovernanceConfigContext.Provider value={query}>
      {children}
    </GovernanceConfigContext.Provider>
  );
}

export function useGovernanceConfigContext(): GovernanceConfigContextValue {
  const value = useContext(GovernanceConfigContext);
  if (value === undefined) {
    throw new Error(
      "useGovernanceConfigContext must be used within a GovernanceConfigProvider",
    );
  }
  return value;
}
