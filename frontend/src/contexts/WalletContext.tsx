'use client';

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from 'react';

import { useIsMobile } from '@/components/connector-kit/hooks/useIsMobile';
import { WalletDrawer } from '@/components/connector-kit/prebuilt/WalletDrawer';
import { WalletModal } from '@/components/connector-kit/prebuilt/WalletModal';

interface WalletContextValue {
  isModalOpen: boolean;
  openModal: () => void;
  closeModal: () => void;
}

const WalletContext = createContext<WalletContextValue | null>(null);

export function WalletProvider({ children }: { children: ReactNode }) {
  const [isModalOpen, setIsModalOpen] = useState(false);
  const isMobile = useIsMobile();

  const openModal = useCallback(() => {
    setIsModalOpen(true);
  }, []);

  const closeModal = useCallback(() => {
    setIsModalOpen(false);
  }, []);

  const handleOpenChange = useCallback((open: boolean) => {
    setIsModalOpen(open);
  }, []);

  const value = useMemo(
    () => ({
      isModalOpen,
      setIsModalOpen: handleOpenChange,
      openModal,
      closeModal,
    }),
    [isModalOpen, handleOpenChange, openModal, closeModal],
  );

  return (
    <WalletContext.Provider value={value}>
      {children}
      {isMobile ? (
        <WalletDrawer
          open={isModalOpen}
          onOpenChange={handleOpenChange}
        />
      ) : (
        <WalletModal
          open={isModalOpen}
          onOpenChange={handleOpenChange}
        />
      )}
    </WalletContext.Provider>
  );
}

export function useWallet() {
  const context = useContext(WalletContext);
  if (!context) {
    throw new Error('useWallet must be used within a WalletProvider');
  }
  return context;
}
