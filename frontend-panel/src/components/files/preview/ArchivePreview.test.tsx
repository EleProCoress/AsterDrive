import {
	act,
	fireEvent,
	render,
	screen,
	waitFor,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ArchivePreview } from "@/components/files/preview/ArchivePreview";
import { ApiError } from "@/services/http";
import type { ArchivePreviewManifest } from "@/types/api";
import { ErrorCode } from "@/types/api-helpers";

vi.mock("@/lib/format", () => ({
	formatBytes: (value: number) => `bytes:${value}`,
	formatDateTime: (value: string) => `time:${value}`,
	formatNumber: (value: number) => `num:${value}`,
}));

vi.mock("react-i18next", () => ({
	useTranslation: () => ({
		t: (key: string, opts?: Record<string, string>) =>
			opts?.date ? `${key}:${opts.date}` : key,
	}),
}));

const manifest: ArchivePreviewManifest = {
	schema_version: 1,
	format: "zip",
	source_blob_id: 10,
	source_hash: "hash",
	generated_at: "2026-01-02T03:04:05Z",
	entry_count: 3,
	file_count: 2,
	directory_count: 1,
	total_uncompressed_size: 12,
	truncated: true,
	entries: [
		{
			path: "docs",
			name: "docs",
			parent: null,
			kind: "directory",
			size: 0,
			compressed_size: 0,
			modified_at: null,
		},
		{
			path: "docs/readme.txt",
			name: "readme.txt",
			parent: "docs",
			kind: "file",
			size: 5,
			compressed_size: 5,
			modified_at: "2026-01-02T03:04:05",
		},
		{
			path: "image.bin",
			name: "image.bin",
			parent: null,
			kind: "file",
			size: 7,
			compressed_size: 7,
			modified_at: null,
		},
	],
};

describe("ArchivePreview", () => {
	it("shows unavailable state without a loader", () => {
		render(<ArchivePreview />);

		expect(screen.getByText("preview_not_available")).toBeInTheDocument();
		expect(screen.getByText("preview_not_available_desc")).toBeInTheDocument();
	});

	it("loads and filters archive entries", async () => {
		const loadManifest = vi.fn(async () => manifest);
		render(<ArchivePreview loadManifest={loadManifest} />);

		expect(await screen.findByText("readme.txt")).toBeInTheDocument();
		expect(screen.getByText("archive_preview_truncated")).toBeInTheDocument();
		expect(screen.getByText("num:3")).toBeInTheDocument();
		expect(screen.getByText("bytes:12")).toBeInTheDocument();

		fireEvent.change(
			screen.getByRole("searchbox", { name: "archive_preview_search" }),
			{ target: { value: "image" } },
		);

		expect(screen.getByText("image.bin")).toBeInTheDocument();
		expect(screen.queryByText("readme.txt")).not.toBeInTheDocument();
	});

	it("shows retry UI when loading fails", async () => {
		const loadManifest = vi.fn(async () => {
			throw new Error("boom");
		});
		render(<ArchivePreview loadManifest={loadManifest} />);

		await waitFor(() => {
			expect(screen.getByText("preview_load_failed")).toBeInTheDocument();
		});
		expect(
			screen.getByRole("button", { name: "preview_retry" }),
		).toBeInTheDocument();
	});

	it("retries loading after a generic failure", async () => {
		const loadManifest = vi
			.fn<() => Promise<ArchivePreviewManifest>>()
			.mockRejectedValueOnce(new Error("boom"))
			.mockResolvedValueOnce(manifest);
		render(<ArchivePreview loadManifest={loadManifest} />);

		fireEvent.click(
			await screen.findByRole("button", { name: "preview_retry" }),
		);

		expect(await screen.findByText("readme.txt")).toBeInTheDocument();
		expect(loadManifest).toHaveBeenCalledTimes(2);
	});

	it("passes an abort signal to the loader and aborts it on unmount", async () => {
		let capturedSignal: AbortSignal | undefined;
		const loadManifest = vi.fn(
			({ signal }: { signal?: AbortSignal } = {}) =>
				new Promise<ArchivePreviewManifest>(() => {
					capturedSignal = signal;
				}),
		);
		const { unmount } = render(<ArchivePreview loadManifest={loadManifest} />);

		await waitFor(() => {
			expect(capturedSignal).toBeInstanceOf(AbortSignal);
		});
		expect(capturedSignal?.aborted).toBe(false);

		unmount();

		expect(capturedSignal?.aborted).toBe(true);
	});

	it("ignores a loader result after unmount", async () => {
		let resolveManifest: (value: ArchivePreviewManifest) => void = () => {};
		const loadManifest = vi.fn(
			() =>
				new Promise<ArchivePreviewManifest>((resolve) => {
					resolveManifest = resolve;
				}),
		);
		const { unmount } = render(<ArchivePreview loadManifest={loadManifest} />);

		unmount();
		await act(async () => {
			resolveManifest(manifest);
		});

		expect(loadManifest).toHaveBeenCalledTimes(1);
	});

	it("shows a friendly disabled state without retry", async () => {
		const loadManifest = vi.fn(async () => {
			throw new ApiError(ErrorCode.Forbidden, "archive preview is disabled", {
				internalCode: "E013",
				subcode: "archive_preview.disabled",
			});
		});
		render(<ArchivePreview loadManifest={loadManifest} />);

		await waitFor(() => {
			expect(screen.getByText("archive_preview_disabled")).toBeInTheDocument();
		});
		expect(
			screen.queryByRole("button", { name: "preview_retry" }),
		).not.toBeInTheDocument();
	});

	it.each([
		"archive_preview.user_disabled",
		"archive_preview.share_disabled",
	])("shows disabled state for %s", async (subcode) => {
		const loadManifest = vi.fn(async () => {
			throw new ApiError(
				ErrorCode.Forbidden,
				"archive preview for this surface is disabled",
				{
					internalCode: "E013",
					subcode,
				},
			);
		});
		render(<ArchivePreview loadManifest={loadManifest} />);

		await waitFor(() => {
			expect(screen.getByText("archive_preview_disabled")).toBeInTheDocument();
		});
		expect(
			screen.queryByRole("button", { name: "preview_retry" }),
		).not.toBeInTheDocument();
	});

	it.each([
		[
			"archive_preview.unsupported_type",
			"archive preview currently supports .zip files only",
			"archive_preview_unsupported_type",
		],
		[
			"archive_preview.source_too_large",
			"source archive size 135064658 exceeds archive preview limit 67108864",
			"archive_preview_source_too_large",
		],
		[
			"archive_preview.invalid_zip",
			"invalid zip archive",
			"archive_preview_invalid_zip",
		],
		[
			"archive_preview.rejected",
			"archive contains 2 entries, exceeds server limit 1",
			"archive_preview_rejected",
		],
		[
			"archive_preview.manifest_too_large",
			"archive preview manifest exceeds server limit",
			"archive_preview_rejected",
		],
		[
			"archive_preview.source_size_mismatch",
			"source archive size mismatch",
			"archive_preview_rejected",
		],
	])("shows a friendly state without retry for %s", async (subcode, message, messageKey) => {
		const loadManifest = vi.fn(async () => {
			throw new ApiError(ErrorCode.BadRequest, message, {
				internalCode: "E005",
				subcode,
			});
		});
		render(<ArchivePreview loadManifest={loadManifest} />);

		await waitFor(() => {
			expect(screen.getByText(messageKey)).toBeInTheDocument();
		});
		expect(
			screen.queryByRole("button", { name: "preview_retry" }),
		).not.toBeInTheDocument();
	});

	it.each([
		[
			"archive preview currently supports .zip files only",
			"archive_preview_unsupported_type",
		],
		[
			"source archive size 135064658 exceeds archive preview limit 67108864",
			"archive_preview_source_too_large",
		],
	])("keeps old-server validation message %s friendly", async (message, messageKey) => {
		const loadManifest = vi.fn(async () => {
			throw new ApiError(ErrorCode.BadRequest, message, {
				internalCode: "E005",
			});
		});
		render(<ArchivePreview loadManifest={loadManifest} />);

		await waitFor(() => {
			expect(screen.getByText(messageKey)).toBeInTheDocument();
		});
		expect(
			screen.queryByRole("button", { name: "preview_retry" }),
		).not.toBeInTheDocument();
	});
});
