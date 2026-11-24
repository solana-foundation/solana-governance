import type { Validator } from "@/types";
import axios from "axios";

const STAKEWIZ_API_URL =
  "https://api.stakewiz.com/validators?sort=-stake_weight";

/** A validator as StakeWiz serves it: every integer is a JSON number. */
export type StakeWizValidator = Omit<
  Validator,
  "activated_stake" | "epoch_credits" | "last_vote"
> & {
  activated_stake: number;
  epoch_credits: number;
  last_vote: number;
};

export const getStakeWizValidators = () =>
  axios.get<StakeWizValidator[]>(STAKEWIZ_API_URL);
