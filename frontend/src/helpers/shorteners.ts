export const shortenPublicKey = (stringPk: string) => {

  return `${stringPk.substring(0, 6)}...${stringPk.substring(
    stringPk.length - 6,
    stringPk.length
  )}`;
};
export const shortenMediumPublickKey = (stringPk: string) => {

  return `${stringPk.substring(0, 10)}...${stringPk.substring(
    stringPk.length - 10,
    stringPk.length
  )}`;
};
