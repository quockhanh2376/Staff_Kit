import { describe, expect, it } from "vitest"
import { toUiColumnDefinition } from "./columnDefinitions"

describe("toUiColumnDefinition", () => {
    it("preserves semantic email metadata while applying a label override", () => {
        expect(
            toUiColumnDefinition(
                {
                    key: "azure_account",
                    label: "Azure AD Account",
                    source: "dynamic",
                    dataType: "email",
                },
                "Directory Account",
            ),
        ).toEqual({
            key: "azure_account",
            label: "Directory Account",
            source: "dynamic",
            dataType: "email",
        })
    })
})
