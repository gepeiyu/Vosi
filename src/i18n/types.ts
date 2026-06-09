export type Locale = "zh" | "en" | "ja";

export const LOCALES: Locale[] = ["zh", "en", "ja"];

export function parseLocale(raw: string): Locale {
  if (raw === "en" || raw === "ja") return raw;
  return "zh";
}
