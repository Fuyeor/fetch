// @app/config/locale.ts

export type SupportedLocale = (typeof supportedLocale)[number];

/**
 * front-end UI supported locales
 */
// prettier-ignore
export const supportedLocale = [
  'en', 'fr', 'es', 'pt', 'de',
  'ar', 'ru', 'ja', 'ko',
  'zh-hans', 'zh-hant'
] as const;
