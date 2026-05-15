import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { formatBytes, formatDateTime, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";
import { ApiError, isRequestCanceled } from "@/services/http";
import type { ArchivePreviewManifest } from "@/types/api";
import { ErrorCode } from "@/types/api-helpers";
import { PreviewError } from "./PreviewError";
import { PreviewLoadingState } from "./PreviewLoadingState";
import { PreviewUnavailable } from "./PreviewUnavailable";

interface ArchivePreviewProps {
	loadManifest?: (options?: {
		signal?: AbortSignal;
	}) => Promise<ArchivePreviewManifest>;
}

type ArchiveEntry = ArchivePreviewManifest["entries"][number];
type ArchivePreviewErrorKind =
	| "disabled"
	| "invalid"
	| "rejected"
	| "sourceTooLarge"
	| "unsupported"
	| "generic";

function classifyArchivePreviewError(error: unknown): ArchivePreviewErrorKind {
	if (!(error instanceof ApiError)) {
		return "generic";
	}

	const subcode = error.subcode ?? "";
	if (
		error.code === ErrorCode.Forbidden &&
		(subcode === "archive_preview.disabled" ||
			subcode === "archive_preview.user_disabled" ||
			subcode === "archive_preview.share_disabled")
	) {
		return "disabled";
	}
	if (
		error.code === ErrorCode.BadRequest &&
		subcode === "archive_preview.unsupported_type"
	) {
		return "unsupported";
	}
	if (
		error.code === ErrorCode.BadRequest &&
		subcode === "archive_preview.source_too_large"
	) {
		return "sourceTooLarge";
	}
	if (
		error.code === ErrorCode.BadRequest &&
		subcode === "archive_preview.invalid_zip"
	) {
		return "invalid";
	}
	if (
		error.code === ErrorCode.BadRequest &&
		(subcode === "archive_preview.rejected" ||
			subcode === "archive_preview.manifest_too_large" ||
			subcode === "archive_preview.source_size_mismatch")
	) {
		return "rejected";
	}

	// Older servers did not attach subcodes yet.
	const message = error.message.toLowerCase();
	if (
		error.code === ErrorCode.Forbidden &&
		message.includes("archive preview") &&
		message.includes("disabled")
	) {
		return "disabled";
	}
	if (
		error.code === ErrorCode.BadRequest &&
		message.includes("archive preview") &&
		message.includes(".zip")
	) {
		return "unsupported";
	}
	if (
		error.code === ErrorCode.BadRequest &&
		message.includes("source archive size") &&
		message.includes("archive preview limit")
	) {
		return "sourceTooLarge";
	}

	return "generic";
}

function entryDepth(path: string) {
	return path.split("/").filter(Boolean).length - 1;
}

function formatZipModifiedAt(value: string | null | undefined) {
	if (!value) return "";
	const normalized = value.endsWith("Z") ? value : `${value}Z`;
	return formatDateTime(normalized);
}

function ArchiveStat({ label, value }: { label: string; value: string }) {
	return (
		<div className="min-w-0 rounded-md border border-border/60 bg-background/70 px-3 py-2 dark:bg-background/20">
			<div className="text-xs text-muted-foreground">{label}</div>
			<div className="truncate text-sm font-medium">{value}</div>
		</div>
	);
}

function ArchiveEntryRow({ entry }: { entry: ArchiveEntry }) {
	const isDirectory = entry.kind === "directory";
	const depth = Math.max(0, entryDepth(entry.path));

	return (
		<div className="grid min-h-10 grid-cols-[minmax(0,1fr)_7.5rem_10rem] items-center gap-3 border-border/50 border-b px-3 text-sm last:border-b-0 max-md:grid-cols-[minmax(0,1fr)_6.5rem]">
			<div className="flex min-w-0 items-center gap-2">
				<span
					className="shrink-0"
					style={{ width: `${Math.min(depth, 8) * 14}px` }}
				/>
				<Icon
					name={isDirectory ? "Folder" : "File"}
					className={cn(
						"h-4 w-4 shrink-0",
						isDirectory ? "text-amber-500" : "text-muted-foreground",
					)}
				/>
				<span className="min-w-0 truncate" title={entry.path}>
					{entry.name}
				</span>
			</div>
			<div className="text-right text-muted-foreground tabular-nums">
				{isDirectory ? "-" : formatBytes(entry.size)}
			</div>
			<div className="truncate text-right text-muted-foreground max-md:hidden">
				{formatZipModifiedAt(entry.modified_at)}
			</div>
		</div>
	);
}

export function ArchivePreview({ loadManifest }: ArchivePreviewProps) {
	const { t } = useTranslation("files");
	const [manifest, setManifest] = useState<ArchivePreviewManifest | null>(null);
	const [query, setQuery] = useState("");
	const [loading, setLoading] = useState(Boolean(loadManifest));
	const [error, setError] = useState<ArchivePreviewErrorKind | null>(null);
	const [reloadKey, setReloadKey] = useState(0);

	useEffect(() => {
		void reloadKey;
		if (!loadManifest) {
			setLoading(false);
			setManifest(null);
			return;
		}

		let cancelled = false;
		const controller = new AbortController();
		setLoading(true);
		setError(null);
		loadManifest({ signal: controller.signal })
			.then((nextManifest) => {
				if (!cancelled) {
					setManifest(nextManifest);
				}
			})
			.catch((error: unknown) => {
				if (!cancelled && !isRequestCanceled(error)) {
					setError(classifyArchivePreviewError(error));
				}
			})
			.finally(() => {
				if (!cancelled) {
					setLoading(false);
				}
			});

		return () => {
			cancelled = true;
			controller.abort();
		};
	}, [loadManifest, reloadKey]);

	const filteredEntries = useMemo(() => {
		if (!manifest) return [];
		const normalized = query.trim().toLowerCase();
		if (!normalized) return manifest.entries;
		return manifest.entries.filter((entry) =>
			entry.path.toLowerCase().includes(normalized),
		);
	}, [manifest, query]);

	if (!loadManifest) {
		return <PreviewUnavailable />;
	}

	if (loading) {
		return (
			<PreviewLoadingState text={t("loading_preview")} className="h-full" />
		);
	}

	if (error || !manifest) {
		if (error === "disabled") {
			return <PreviewError messageKey="archive_preview_disabled" />;
		}
		if (error === "unsupported") {
			return <PreviewError messageKey="archive_preview_unsupported_type" />;
		}
		if (error === "sourceTooLarge") {
			return <PreviewError messageKey="archive_preview_source_too_large" />;
		}
		if (error === "invalid") {
			return <PreviewError messageKey="archive_preview_invalid_zip" />;
		}
		if (error === "rejected") {
			return <PreviewError messageKey="archive_preview_rejected" />;
		}

		return <PreviewError onRetry={() => setReloadKey((value) => value + 1)} />;
	}

	return (
		<div className="flex h-full min-h-0 w-full min-w-0 flex-col overflow-hidden rounded-xl border border-border/70 bg-card shadow-xs dark:shadow-none">
			<div className="flex flex-wrap items-center justify-between gap-2 border-border/60 border-b bg-muted/20 px-4 py-3 dark:bg-muted/15">
				<div className="flex min-w-0 items-center gap-2">
					<Icon name="FileZip" className="h-5 w-5 shrink-0 text-amber-500" />
					<div className="min-w-0">
						<div className="truncate text-sm font-medium">
							{t("archive_preview_title")}
						</div>
						<div className="truncate text-xs text-muted-foreground">
							{t("archive_preview_generated_at", {
								date: formatDateTime(manifest.generated_at),
							})}
						</div>
					</div>
				</div>
				{manifest.truncated ? (
					<Badge variant="outline">{t("archive_preview_truncated")}</Badge>
				) : null}
			</div>

			<div className="grid shrink-0 grid-cols-2 gap-2 border-border/60 border-b p-3 md:grid-cols-4">
				<ArchiveStat
					label={t("archive_preview_entries")}
					value={formatNumber(manifest.entry_count)}
				/>
				<ArchiveStat
					label={t("archive_preview_files")}
					value={formatNumber(manifest.file_count)}
				/>
				<ArchiveStat
					label={t("archive_preview_folders")}
					value={formatNumber(manifest.directory_count)}
				/>
				<ArchiveStat
					label={t("archive_preview_uncompressed")}
					value={formatBytes(manifest.total_uncompressed_size)}
				/>
			</div>

			<div className="flex shrink-0 items-center gap-2 border-border/60 border-b px-3 py-2">
				<Icon
					name="MagnifyingGlass"
					className="h-4 w-4 text-muted-foreground"
				/>
				<Input
					type="search"
					aria-label={t("archive_preview_search")}
					placeholder={t("archive_preview_search")}
					value={query}
					onChange={(event) => setQuery(event.target.value)}
					className="h-7 border-transparent bg-transparent px-1 shadow-none focus-visible:bg-transparent focus-visible:ring-0"
				/>
				{query ? (
					<Button
						type="button"
						variant="ghost"
						size="icon-xs"
						aria-label={t("archive_preview_clear_search")}
						onClick={() => setQuery("")}
					>
						<Icon name="X" className="h-3.5 w-3.5" />
					</Button>
				) : null}
			</div>

			<div className="grid h-8 shrink-0 grid-cols-[minmax(0,1fr)_7.5rem_10rem] items-center gap-3 border-border/60 border-b bg-muted/20 px-3 text-xs font-medium text-muted-foreground max-md:grid-cols-[minmax(0,1fr)_6.5rem] dark:bg-muted/15">
				<div>{t("archive_preview_name")}</div>
				<div className="text-right">{t("archive_preview_size")}</div>
				<div className="text-right max-md:hidden">
					{t("archive_preview_modified")}
				</div>
			</div>

			<div className="min-h-0 flex-1">
				{filteredEntries.length > 0 ? (
					<ScrollArea className="h-full bg-background/80 dark:bg-background/25">
						{filteredEntries.map((entry) => (
							<ArchiveEntryRow
								key={`${entry.kind}:${entry.path}:${entry.size}`}
								entry={entry}
							/>
						))}
					</ScrollArea>
				) : (
					<div className="flex h-full min-h-[12rem] items-center justify-center px-6 text-center text-sm text-muted-foreground">
						{query
							? t("archive_preview_no_matches")
							: t("archive_preview_empty")}
					</div>
				)}
			</div>
		</div>
	);
}
