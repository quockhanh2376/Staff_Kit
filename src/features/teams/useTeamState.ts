import { useEffect, useState } from "react"
import { staffApi } from "../../services/staff-api"
import type { TeamRecord } from "../../types/staff"
import { getErrorMessage } from "../../lib/utils"

type UseTeamStateOptions = {
    dbReady: boolean
    isAuthenticated: boolean
    reloadToken: number
    setGlobalError: (msg: string | null) => void
    triggerReload: () => void
}

export type TeamState = ReturnType<typeof useTeamState>

export function useTeamState({
    dbReady,
    isAuthenticated,
    reloadToken,
    setGlobalError,
    triggerReload,
}: UseTeamStateOptions) {
    const [teams, setTeams] = useState<TeamRecord[]>([])
    const [newTeamName, setNewTeamName] = useState("")
    const [newTeamParentName, setNewTeamParentName] = useState("")
    const [isSavingTeam, setSavingTeam] = useState(false)
    const [isLoadingTeams, setLoadingTeams] = useState(false)

    useEffect(() => {
        if (!dbReady || !isAuthenticated) return

        let disposed = false

        void (async () => {
            try {
                setLoadingTeams(true)
                const data = await staffApi.listTeams()
                if (!disposed) setTeams(data)
            } catch (error) {
                if (!disposed) setGlobalError(getErrorMessage(error))
            } finally {
                if (!disposed) setLoadingTeams(false)
            }
        })()

        return () => { disposed = true }
    }, [dbReady, isAuthenticated, reloadToken, setGlobalError])

    const handleCreateTeam = async () => {
        const name = newTeamName.trim()
        if (!name || isSavingTeam) return
        try {
            setSavingTeam(true)
            await staffApi.upsertTeam({
                name,
                parentName: newTeamParentName.trim() || null,
            })
            setNewTeamName("")
            setNewTeamParentName("")
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        } finally {
            setSavingTeam(false)
        }
    }

    const handleRenameTeam = async (team: TeamRecord) => {
        const nextName = window.prompt("Rename team", team.name)
        if (!nextName || nextName.trim() === "") return
        try {
            await staffApi.upsertTeam({
                id: team.id,
                name: nextName.trim(),
                parentName: team.parentName,
            })
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        }
    }

    const handleSetParent = async (team: TeamRecord, parentName: string | null) => {
        try {
            await staffApi.upsertTeam({
                id: team.id,
                name: team.name,
                parentName: parentName,
            })
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        }
    }

    const handleDeleteTeam = async (team: TeamRecord) => {
        const accepted = window.confirm(
            `Delete team '${team.name}'? Employees in this team will have their team cleared.`,
        )
        if (!accepted) return
        try {
            await staffApi.deleteTeam(team.id)
            triggerReload()
        } catch (error) {
            setGlobalError(getErrorMessage(error))
        }
    }

    return {
        teams,
        newTeamName,
        setNewTeamName,
        newTeamParentName,
        setNewTeamParentName,
        isSavingTeam,
        isLoadingTeams,
        handleCreateTeam,
        handleRenameTeam,
        handleSetParent,
        handleDeleteTeam,
    }
}
