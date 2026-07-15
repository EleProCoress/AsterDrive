import { lazy, Suspense } from "react";
import { Navigate, useParams } from "react-router-dom";
import { PERSONAL_WORKSPACE, type Workspace } from "@/lib/workspace";
import { Loading } from "./Loading";

const WorkspaceOutlet = lazy(() =>
	import("./WorkspaceOutlet").then((module) => ({
		default: module.WorkspaceOutlet,
	})),
);

export function WorkspaceRoute() {
	const { teamId } = useParams<{ teamId?: string }>();
	let workspace: Workspace = PERSONAL_WORKSPACE;

	if (teamId !== undefined) {
		if (!/^[1-9]\d*$/.test(teamId)) {
			return <Navigate to="/" replace />;
		}
		const parsedTeamId = Number(teamId);
		if (!Number.isSafeInteger(parsedTeamId)) {
			return <Navigate to="/" replace />;
		}
		workspace = { kind: "team", teamId: parsedTeamId };
	}

	return (
		<Suspense fallback={<Loading />}>
			<WorkspaceOutlet workspace={workspace} />
		</Suspense>
	);
}
