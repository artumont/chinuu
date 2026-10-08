import { chinuuLight } from "./chinuuLight.ts";
import { chinuuDark } from "./chinuuDark.ts";
import {
  tokyoNight,
  tokyoNightLight,
  tokyoNightStorm,
} from "./tokyoNight/index.ts";

export { chinuuLight } from "./chinuuLight.ts";
export { chinuuDark } from "./chinuuDark.ts";
export {
  tokyoNight,
  tokyoNightLight,
  tokyoNightStorm,
  tokyoNightVariants,
} from "./tokyoNight/index.ts";

/**
 * Shipped themes, addressable by name.
 *
 * `tokyoNight` is the default dark variant; Storm and Light are the other two
 * Tokyo Night variants.
 */
export const builtinThemes = {
  chinuuLight,
  chinuuDark,
  tokyoNight,
  tokyoNightStorm,
  tokyoNightLight,
} as const;
