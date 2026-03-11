---
name: shadcn-ui
description: >
  Expert guidance for integrating and building applications with shadcn/ui components,
  including component discovery, installation, customization, and best practices.
---

# shadcn/ui — Staff Kit Web App

## Setup

```bash
npx shadcn@latest init
# Choose: TypeScript, Tailwind, App Router, src/ directory
```

## Component installation

```bash
# Install only what you need — don't install all
npx shadcn@latest add button
npx shadcn@latest add input
npx shadcn@latest add table
npx shadcn@latest add dialog
npx shadcn@latest add dropdown-menu
npx shadcn@latest add select
npx shadcn@latest add badge
npx shadcn@latest add toast
npx shadcn@latest add alert
npx shadcn@latest add skeleton
npx shadcn@latest add tooltip
npx shadcn@latest add form          # includes react-hook-form integration
```

## Components used in Staff Kit

| Feature | shadcn components |
|---------|-------------------|
| Employee table | `Table`, `Badge`, `Tooltip`, `DropdownMenu` |
| Login form | `Form`, `Input`, `Button` |
| Import Excel | `Dialog`, `Progress`, `Alert` |
| Settings | `Tabs`, `Card`, `Select`, `Badge` |
| Role editor | `Select`, `Dialog` |
| Backup | `Button`, `Alert` |
| Loading states | `Skeleton` |
| Notifications | `Toast` (via Sonner) |

## Customization

Components are **copied to `src/components/ui/`** — edit freely:

```typescript
// src/components/ui/badge.tsx — customize variant colors
const badgeVariants = cva("...", {
  variants: {
    variant: {
      default: "bg-primary",
      admin: "bg-blue-500 text-white",          // custom Staff Kit role badge
      superAdmin: "bg-purple-600 text-white",   // custom
      user: "bg-gray-200 text-gray-700",        // custom
    },
  },
})
```

## Don't override component files after customization

Once you've customized a component, don't re-run `npx shadcn@latest add` for that
component — it will overwrite your changes.

## DO NOT

- Do NOT install shadcn components via `npm install` — always use `npx shadcn@latest add`
- Do NOT import from `shadcn/ui` directly — import from `@/components/ui/xxx`
- Do NOT restyle shadcn primitives with arbitrary Tailwind in the feature components — extend at the ui layer
