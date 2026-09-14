import de from "../locales/de.json";
import en from "../locales/en.json";
export type Locale = "de" | "en";
export type Language = "system" | Locale;
export type TextKey = keyof typeof en;
export type Message = {
	key: string;
	params?: Record<string, string | number>;
	causes?: Message[];
};
export function t(
	locale: Locale,
	key: TextKey,
	params: Record<string, string | number> = {},
): string {
	const template =
		({ de, en }[locale] as Record<string, string>)[key] ?? en[key] ?? key;
	return template.replace(/\{([^{}]+)\}/g, (token, name) =>
		String(params[name] ?? token),
	);
}
export function renderMessage(locale: Locale, message: Message): string {
	const body = t(locale, message.key as TextKey, message.params);
	return (
		body +
		(message.causes?.length
			? `: ${message.causes.map((c) => renderMessage(locale, c)).join("; ")}`
			: "")
	);
}
export function commandError(error: unknown): Message {
	const cause =
		typeof error === "object" && error !== null && "key" in error
			? (error as Message)
			: { key: "error.technical", params: { details: String(error) } };
	return { key: "error.command", causes: [cause] };
}
