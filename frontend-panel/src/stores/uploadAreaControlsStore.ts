import type { DragEvent } from "react";
import { create } from "zustand";

export interface UploadAreaControls {
	isDragging: boolean;
	handleDragEnter: (event: DragEvent<HTMLDivElement>) => void;
	handleDragLeave: (event: DragEvent<HTMLDivElement>) => void;
	handleDragOver: (event: DragEvent<HTMLDivElement>) => void;
	handleDrop: (event: DragEvent<HTMLDivElement>) => Promise<void>;
	triggerFileUpload: () => void;
	triggerFolderUpload: () => void;
}

interface UploadAreaControlsState {
	controls: UploadAreaControls | null;
	setControls: (controls: UploadAreaControls | null) => void;
}

export const useUploadAreaControlsStore = create<UploadAreaControlsState>(
	(set) => ({
		controls: null,
		setControls: (controls) => set({ controls }),
	}),
);
