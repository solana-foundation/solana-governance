export type Validators = Validator[];

export interface Validator {
  name: string;
  /** Validator identity (withdrawal authority). Used by governance votes. */
  identity?: string;
  vote_identity: string;
  activated_stake: bigint;
  image?: string | null;
  version?: string;
  commission: number;
  asn?: string;
  description?: string;
  stake_weight?: number;
  credits?: number;
  epoch_credits: bigint;
  last_vote: bigint;

  // Not used in the frontend but can be mapped if needed in the future
  // rank: number;
  // identity: string;
  // root_slot: number;
  // version: string;
  // delinquent: boolean;
  // skip_rate: number;
  // updated_at: string;
  // first_epoch_with_stake: number;
  // name: string;
  // keybase: string;

  // website: string;
  // image: string | null;
  // ip_latitude: string;
  // ip_longitude: string;
  // ip_city: string;
  // ip_country: string;
  // ip_asn: string;
  // ip_org: string;
  // mod: boolean;
  // is_jito: boolean;
  // jito_commission_bps: number;
  // vote_success: number;
  // vote_success_score: number;
  // skip_rate_score: number;
  // info_score: number;
  // commission_score: number;
  // first_epoch_distance: number;
  // epoch_distance_score: number;
  // above_halt_line: boolean;
  // stake_weight_score: number;
  // withdraw_authority_score: number;
  // asn: string;
  // asn_concentration: number;
  // asn_concentration_score: number;
  // uptime: number;
  // uptime_score: number;
  // wiz_score: number;
  // version_valid: boolean;
  // city_concentration: number;
  // city_concentration_score: number;
  // invalid_version_score: number;
  // superminority_penalty: number;
  // score_version: number;
  // no_voting_override: boolean;
  // epoch: number;
  // epoch_slot_height: number;
  // asncity_concentration: number;
  // asncity_concentration_score: number;
  // stake_ratio: number;
  // credit_ratio: number;
  // apy_estimate: number;
}
