import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { STORAGE_KEYS } from "@/config/app";
import TrashPage from "@/pages/TrashPage";

const mockState = vi.hoisted(() => ({
	formatBatchToast: vi.fn((_: unknown, operation: string) => ({
		title: `toast:${operation}`,
		variant: "success",
	})),
	handleApiError: vi.fn(),
	list: vi.fn(),
	purgeAll: vi.fn(),
	purgeFile: vi.fn(),
	purgeFolder: vi.fn(),
	refreshUser: vi.fn(),
	restoreFile: vi.fn(),
	restoreFolder: vi.fn(),
	selectionShortcuts: vi.fn(),
	toastError: vi.fn(),
	toastSuccess: vi.fn(),
}));

vi.mock("react-i18next", () => ({
	useTranslation: () => ({
		t: (key: string, opts?: Record<string, unknown>) => {
			if (key === "selected_count") return `selected:${opts?.count}`;
			if (key === "items_count") return `items:${opts?.count}`;
			if (key === "files:trash_purge_confirm_title") {
				return `purge-title:${opts?.count}`;
			}
			return key;
		},
	}),
}));

vi.mock("sonner", () => ({
	toast: {
		error: (...args: unknown[]) => mockState.toastError(...args),
		success: (...args: unknown[]) => mockState.toastSuccess(...args),
	},
}));

vi.mock("@/components/common/ConfirmDialog", () => ({
	ConfirmDialog: ({
		open,
		title,
		description,
		confirmLabel,
		onConfirm,
		onOpenChange,
	}: {
		open: boolean;
		title: string;
		description: string;
		confirmLabel: string;
		onConfirm: () => void;
		onOpenChange: (open: boolean) => void;
	}) =>
		open ? (
			<div data-testid="confirm-dialog">
				<h2>{title}</h2>
				<p>{description}</p>
				<button type="button" onClick={onConfirm}>
					{confirmLabel}
				</button>
				<button type="button" onClick={() => onOpenChange(false)}>
					close-confirm
				</button>
			</div>
		) : null,
}));

vi.mock("@/components/common/EmptyState", () => ({
	EmptyState: ({
		title,
		description,
	}: {
		title: string;
		description: string;
	}) => <div>{`${title}:${description}`}</div>,
}));

vi.mock("@/components/common/SkeletonFileGrid", () => ({
	SkeletonFileGrid: () => <div>skeleton-grid</div>,
}));

vi.mock("@/components/common/SkeletonFileTable", () => ({
	SkeletonFileTable: () => <div>skeleton-table</div>,
}));

vi.mock("@/components/common/ViewToggle", () => ({
	ViewToggle: ({
		value,
		onChange,
	}: {
		value: string;
		onChange: (value: "grid" | "list") => void;
	}) => (
		<div>
			<div>{`view:${value}`}</div>
			<button type="button" onClick={() => onChange("grid")}>
				grid
			</button>
			<button type="button" onClick={() => onChange("list")}>
				list
			</button>
		</div>
	),
}));

vi.mock("@/components/layout/AppLayout", () => ({
	AppLayout: ({
		actions,
		children,
	}: {
		actions?: React.ReactNode;
		children: React.ReactNode;
	}) => (
		<div data-testid="app-layout">
			<div>{actions}</div>
			{children}
		</div>
	),
}));

vi.mock("@/components/trash/TrashBatchActionBar", () => ({
	TrashBatchActionBar: ({
		count,
		onRestore,
		onPurge,
		onClearSelection,
	}: {
		count: number;
		onRestore: () => void;
		onPurge: () => void;
		onClearSelection: () => void;
	}) =>
		count > 0 ? (
			<div>
				<div>{`batch-count:${count}`}</div>
				<button type="button" onClick={onRestore}>
					restore-selected
				</button>
				<button type="button" onClick={onPurge}>
					purge-selected
				</button>
				<button type="button" onClick={onClearSelection}>
					clear-selection
				</button>
			</div>
		) : null,
}));

vi.mock("@/components/trash/TrashGrid", () => ({
	TrashGrid: ({
		items,
		onToggleSelect,
		onRestore,
		onPurge,
	}: {
		items: Array<{ id: number; name: string }>;
		onToggleSelect: (item: never) => void;
		onRestore: (item: never) => void;
		onPurge: (item: never) => void;
	}) => (
		<div>
			{items.map((item) => (
				<div key={item.id}>
					<button type="button" onClick={() => onToggleSelect(item as never)}>
						{`select:${item.name}`}
					</button>
					<button type="button" onClick={() => onRestore(item as never)}>
						{`restore:${item.name}`}
					</button>
					<button type="button" onClick={() => onPurge(item as never)}>
						{`purge:${item.name}`}
					</button>
				</div>
			))}
		</div>
	),
}));

vi.mock("@/components/trash/TrashTable", () => ({
	TrashTable: ({
		items,
		onToggleSelectAll,
		onToggleSelect,
		onRestore,
		onPurge,
	}: {
		items: Array<{ id: number; name: string }>;
		onToggleSelectAll: () => void;
		onToggleSelect: (item: never) => void;
		onRestore: (item: never) => void;
		onPurge: (item: never) => void;
	}) => (
		<div>
			<button type="button" onClick={onToggleSelectAll}>
				toggle-all
			</button>
			{items.map((item) => (
				<div key={item.id}>
					<button type="button" onClick={() => onToggleSelect(item as never)}>
						{`select:${item.name}`}
					</button>
					<button type="button" onClick={() => onRestore(item as never)}>
						{`restore:${item.name}`}
					</button>
					<button type="button" onClick={() => onPurge(item as never)}>
						{`purge:${item.name}`}
					</button>
				</div>
			))}
		</div>
	),
}));

vi.mock("@/components/ui/button", () => ({
	Button: ({
		children,
		type,
		disabled,
		onClick,
		className,
		title,
	}: {
		children: React.ReactNode;
		type?: "button" | "submit";
		disabled?: boolean;
		onClick?: () => void;
		className?: string;
		title?: string;
	}) => (
		<button
			type={type ?? "button"}
			disabled={disabled}
			onClick={onClick}
			className={className}
			title={title}
		>
			{children}
		</button>
	),
}));

vi.mock("@/components/ui/icon", () => ({
	Icon: ({ name }: { name: string }) => <span>{name}</span>,
}));

vi.mock("@/components/ui/item-checkbox", () => ({
	ItemCheckbox: ({
		checked,
		onChange,
	}: {
		checked: boolean;
		onChange: () => void;
	}) => (
		<button
			type="button"
			aria-label={`checkbox:${checked ? "checked" : "unchecked"}`}
			onClick={onChange}
		/>
	),
}));

vi.mock("@/components/ui/scroll-area", () => ({
	ScrollArea: ({
		children,
		className,
	}: {
		children: React.ReactNode;
		className?: string;
	}) => <div className={className}>{children}</div>,
}));

vi.mock("@/hooks/useApiError", () => ({
	handleApiError: (...args: unknown[]) => mockState.handleApiError(...args),
}));

vi.mock("@/hooks/useSelectionShortcuts", () => ({
	useSelectionShortcuts: (...args: unknown[]) =>
		mockState.selectionShortcuts(...args),
}));

vi.mock("@/lib/formatBatchToast", () => ({
	formatBatchToast: (...args: unknown[]) => mockState.formatBatchToast(...args),
}));

vi.mock("@/services/trashService", () => ({
	trashService: {
		list: (...args: unknown[]) => mockState.list(...args),
		purgeAll: (...args: unknown[]) => mockState.purgeAll(...args),
		purgeFile: (...args: unknown[]) => mockState.purgeFile(...args),
		purgeFolder: (...args: unknown[]) => mockState.purgeFolder(...args),
		restoreFile: (...args: unknown[]) => mockState.restoreFile(...args),
		restoreFolder: (...args: unknown[]) => mockState.restoreFolder(...args),
	},
}));

vi.mock("@/stores/authStore", () => ({
	useAuthStore: (
		selector: (state: { refreshUser: typeof mockState.refreshUser }) => unknown,
	) => selector({ refreshUser: mockState.refreshUser }),
}));

const fileItem = {
	entity_type: "file",
	expires_at: "2026-04-08T00:00:00Z",
	id: 1,
	name: "report.pdf",
	original_path: "/Docs",
	size: 12,
} as never;

function emptyTrashContents() {
	return {
		files: [],
		files_total: 0,
		folders: [],
		folders_total: 0,
		next_file_cursor: null,
	} as never;
}

describe("TrashPage", () => {
	beforeEach(() => {
		localStorage.clear();
		mockState.formatBatchToast.mockClear();
		mockState.handleApiError.mockReset();
		mockState.list.mockReset();
		mockState.purgeAll.mockReset();
		mockState.purgeFile.mockReset();
		mockState.purgeFolder.mockReset();
		mockState.refreshUser.mockReset();
		mockState.restoreFile.mockReset();
		mockState.restoreFolder.mockReset();
		mockState.selectionShortcuts.mockReset();
		mockState.toastError.mockReset();
		mockState.toastSuccess.mockReset();

		mockState.list.mockResolvedValue(emptyTrashContents());
		mockState.purgeAll.mockResolvedValue(undefined);
		mockState.purgeFile.mockResolvedValue(undefined);
		mockState.purgeFolder.mockResolvedValue(undefined);
		mockState.refreshUser.mockResolvedValue(undefined);
		mockState.restoreFile.mockResolvedValue(undefined);
		mockState.restoreFolder.mockResolvedValue(undefined);
	});

	it("uses the stored grid preference and persists view mode changes", async () => {
		localStorage.setItem(STORAGE_KEYS.trashViewMode, "grid");

		render(<TrashPage />);

		expect(
			await screen.findByText("files:trash_empty_title:files:trash_empty_desc"),
		).toBeInTheDocument();
		expect(screen.getByText("view:grid")).toBeInTheDocument();

		fireEvent.click(screen.getByRole("button", { name: "list" }));

		expect(localStorage.getItem(STORAGE_KEYS.trashViewMode)).toBe("list");
		expect(screen.getByText("view:list")).toBeInTheDocument();
	});

	it("restores selected items through the batch action bar and reloads the list", async () => {
		mockState.list
			.mockResolvedValueOnce({
				files: [fileItem],
				files_total: 1,
				folders: [],
				folders_total: 0,
				next_file_cursor: null,
			} as never)
			.mockResolvedValueOnce(emptyTrashContents());

		render(<TrashPage />);

		await screen.findByText("select:report.pdf");
		fireEvent.click(screen.getByRole("button", { name: "select:report.pdf" }));

		expect(screen.getByText("batch-count:1")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "restore-selected" }));

		await waitFor(() => {
			expect(mockState.restoreFile).toHaveBeenCalledWith(1);
		});
		expect(mockState.formatBatchToast).toHaveBeenCalledWith(
			expect.any(Function),
			"restore",
			{
				errors: [],
				failed: 0,
				succeeded: 1,
			},
		);
		expect(mockState.toastSuccess).toHaveBeenCalledWith("toast:restore");
		await waitFor(() => {
			expect(mockState.refreshUser).toHaveBeenCalledTimes(1);
		});
	});

	it("confirms and empties the trash, then reloads contents", async () => {
		mockState.list
			.mockResolvedValueOnce({
				files: [fileItem],
				files_total: 1,
				folders: [],
				folders_total: 0,
				next_file_cursor: null,
			} as never)
			.mockResolvedValueOnce(emptyTrashContents());

		render(<TrashPage />);

		await screen.findByRole("button", { name: "select:report.pdf" });
		fireEvent.click(screen.getByRole("button", { name: /admin:empty_trash/i }));

		expect(await screen.findByText("are_you_sure")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "admin:empty_trash" }));

		await waitFor(() => {
			expect(mockState.purgeAll).toHaveBeenCalledTimes(1);
		});
		expect(mockState.toastSuccess).toHaveBeenCalledWith("trash_emptied");
		await waitFor(() => {
			expect(mockState.refreshUser).toHaveBeenCalledTimes(1);
		});
	});
});
