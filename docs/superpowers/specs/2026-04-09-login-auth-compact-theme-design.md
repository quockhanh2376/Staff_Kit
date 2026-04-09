# Login Auth Compact Theme Design

**Date:** 2026-04-09

**Owner:** Cheki

## Goal

Redesign the `Login` and `Forgot Password` experience so it visually matches the compact dark Settings system while keeping the current authentication flow and theme toggle behavior intact.

## Scope

- Update the shared auth shell in `LoginPage.tsx`
- Apply a compact card style for both `Login` and `Forgot Password`
- Keep the `Staff Kit` header and `Dark / Light` toggle
- Support both dark mode and light mode with the same layout
- Keep existing auth state, submit handlers, loading states, and error handling

## Out Of Scope

- Backend authentication logic
- Password reset business rules
- Session handling
- Account/role permissions
- Settings page layout changes

## Design Summary

### Auth Shell

The screen keeps the current top header with the `Staff Kit` brand and the existing theme toggle. The auth content below stays centered vertically and horizontally, but the card becomes visually tighter and closer to the reference mock.

### Visual System

The auth screen should align with the same visual language already used by the compact Settings work:

- Dark page background close to `#0d1117`
- Dark card surface close to `#161b22`
- Slate borders and muted secondary text
- Emerald primary action buttons
- Outline or ghost secondary action buttons
- Inputs darker than the card surface with emerald focus state

Light mode uses the same structure and spacing, but swaps to light surfaces and dark slate text while keeping emerald as the primary accent.

### Login Card

The login card stays a single centered panel around `420px` wide.

It contains:

- Title: `Login`
- Subtitle: `Welcome to Staff Kit ! Please Sign In`
- Username field
- Password field with visibility toggle
- Primary `Login` button
- Secondary `Forgot password` button

Spacing should be compact and consistent with the reference mock. Labels stay uppercase and small. Inputs and buttons should visually match the compact Settings surfaces.

### Forgot Password Card

Forgot password remains a mode inside the same component, not a separate page.

It reuses the same shell and swaps content to:

- Title: `Forgot Password`
- Subtitle explaining recovery flow
- Username field
- Recovery code field
- New password field with visibility toggle
- Primary `Reset Password` button
- Secondary `Back to login` button

The shared shell keeps transitions simple and avoids duplicate layout code.

### Error And Loading States

Existing loading banners and error messages remain, but should inherit the new compact spacing and surfaces so they do not visually clash with the auth card.

## Component Boundaries

- `src/features/auth/LoginPage.tsx`
  - Owns the auth shell, layout, and card styling
  - Renders both login and forgot-password modes
- `src/features/auth/useAuthState.ts`
  - No behavior change expected
- `src/index.css`
  - Reuse existing theme tokens and form-input rules where possible
  - Add auth-specific utility classes only if `LoginPage.tsx` becomes too dense

## Risks

- Over-fitting to the dark mock can make light mode feel inconsistent
- Reworking too many global tokens would risk changing other screens
- Adding too many auth-specific classes to `index.css` could create styling drift if not scoped carefully

## Chosen Approach

Keep the existing app header and auth state structure, but redesign the auth card in `LoginPage.tsx` so both modes match the compact Settings tone without changing auth logic or global theme behavior.
