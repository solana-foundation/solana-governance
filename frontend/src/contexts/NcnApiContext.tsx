"use client";

import React, { createContext, useContext, useState, ReactNode } from "react";
import { DEFAULT_NCN_API_URL } from "@/lib/constants";

interface NcnApiContextType {
  ncnApiUrl: string;
  setNcnApiUrl: (url: string) => void;
  resetToDefault: () => void;
}

const NcnApiContext = createContext<NcnApiContextType | undefined>(undefined);

const STORAGE_KEY = "ncn-api-url";

const normalizeUrl = (url: string): string => {
  return url.replace(/\/$/, "");
};

const getStoredValue = (): string => {
  if (typeof window !== "undefined") {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      try {
        return normalizeUrl(saved);
      } catch {
        console.error("error parsing ncn api url from local storage");
        return DEFAULT_NCN_API_URL;
      }
    }
  }
  return DEFAULT_NCN_API_URL;
};

export function NcnApiProvider({ children }: { children: ReactNode }) {
  const [ncnApiUrl, setNcnApiUrlState] = useState<string>(getStoredValue());

  const setNcnApiUrlData = (url: string) => {
    const normalized = normalizeUrl(url);
    setNcnApiUrlState(normalized);
    localStorage.setItem(STORAGE_KEY, normalized);
  };

  const resetToDefault = () => {
    setNcnApiUrlState(DEFAULT_NCN_API_URL);
    localStorage.removeItem(STORAGE_KEY);
  };

  return (
    <NcnApiContext.Provider
      value={{
        ncnApiUrl,
        setNcnApiUrl: setNcnApiUrlData,
        resetToDefault,
      }}
    >
      {children}
    </NcnApiContext.Provider>
  );
}

export function useNcnApi() {
  const context = useContext(NcnApiContext);
  if (context === undefined) {
    throw new Error("useNcnApi must be used within an NcnApiProvider");
  }
  return context;
}
