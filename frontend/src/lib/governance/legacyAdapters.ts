/** Temporary UI value adapters while presentation components retain their old shapes. */
export class LegacyPublicKey {
  constructor(private readonly address: string) {}
  toBase58() { return this.address; }
  toString() { return this.address; }
}

export class LegacyBn {
  constructor(private readonly value: bigint) {}
  toNumber() { return Number(this.value); }
  toString() { return this.value.toString(); }
}

export const toLegacyPublicKey = (address: string) => new LegacyPublicKey(address);
export const toLegacyBn = (value: bigint) => new LegacyBn(value);
