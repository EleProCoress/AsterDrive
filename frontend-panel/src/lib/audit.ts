import type { TFunction } from "i18next";
import type { AuditAction } from "@/types/api";

export const AUDIT_ENTITY_TYPE_FILTER_VALUES = [
	"file",
	"folder",
	"team",
	"user",
	"share",
	"task",
	"resource_lock",
	"storage_policy",
	"policy_group",
	"system_config",
	"remote_node",
	"remote_ingress_profile",
	"webdav_account",
	"upload_session",
	"stream_ticket",
	"auth_session",
	"trash",
] as const;

function resolveAuditTranslation(
	t: TFunction,
	key: string,
	ns: "admin" | "settings",
	fallback?: string,
) {
	const translated = t(key, { ns, defaultValue: key });
	return translated === key ? fallback : translated;
}

export function formatAuditAction(t: TFunction, action: AuditAction | string) {
	const value = String(action);
	return (
		resolveAuditTranslation(t, `audit_action_${value}`, "admin") ??
		resolveAuditTranslation(t, value, "settings", value) ??
		value
	);
}

export function formatAuditEntityType(
	t: TFunction,
	entityType: string | null | undefined,
) {
	if (!entityType) {
		return "---";
	}

	return (
		resolveAuditTranslation(
			t,
			`audit_entity_type_${entityType}`,
			"admin",
			entityType,
		) ?? entityType
	);
}
