import { Validators } from "@/types";
import axios from "axios";

const STAKEWIZ_API_URL =
  "https://api.stakewiz.com/validators?sort=-stake_weight";

export const getStakeWizValidators = () =>
  axios.get<Validators>(STAKEWIZ_API_URL);
