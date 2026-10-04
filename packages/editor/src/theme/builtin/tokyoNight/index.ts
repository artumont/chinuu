import { tokyoNightLight } from "./light.ts";
import { tokyoNight } from "./night.ts";
import { tokyoNightStorm } from "./storm.ts";

export { tokyoNightLight } from "./light.ts";
export { tokyoNight } from "./night.ts";
export { tokyoNightStorm } from "./storm.ts";

/**
 * The three Tokyo Night variants that ship upstream.
 *
 * `moon` also exists in `folke/tokyonight.nvim` but is a separate design rather
 * than a variant of this set, so it is not included.
 */
export const tokyoNightVariants = {
  night: tokyoNight,
  storm: tokyoNightStorm,
  light: tokyoNightLight,
} as const;
