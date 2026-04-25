# v0.7.0 Phase 4: UI Polish

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enforce consistent UI standards across all pages — skeleton loading, pagination defaults, toast notifications, form validation, confirmation dialogs, and empty state guidance.

**Architecture:** Three tracks — (A) global standards enforcement, (B) page-specific improvements, (C) universal UX patterns.

**Tech Stack:** React 18, TypeScript, Tailwind CSS, Zustand, Radix UI, Sonner/useToast

**Spec:** `docs/superpowers/specs/2026-04-26-v0.7.0-release-plan-design.md` Part 2

**Depends on:** Phase 2 (alert() replacement, console cleanup should be done first)

---

## Key References

- **Toast**: `useToast()` from `@/hooks/use-toast` (Radix UI based, TOAST_LIMIT=1, very long duration)
- **Skeleton**: `LoadingState` component in `web/src/components/shared/LoadingState.tsx` with `variant="page"` for page-level loading
- **Pagination**: `PaginatedContent` component with `DEFAULT_PAGE_SIZE = 10` in `web/src/components/shared/PaginatedContent.tsx`
- **Page Layout**: `PageLayout` with `PageTabsBar`/`PageTabsContent` pattern

---

## Track A: Global Standards Enforcement

### Task A1: Fix Inconsistent Pagination Defaults

**Files:**
- Modify: `web/src/hooks/useApiData.ts` (default pageSize = 20 → 10)
- Modify: `web/src/lib/api.ts` (listSessions default pageSize = 20 → 10)
- Modify: `web/src/store/slices/sessionSlice.ts` (pageSize = 50 → 10)

- [ ] **Step 1: Audit all pagination values**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind/web && grep -rn 'pageSize\|per_page\|PAGE_SIZE' src/ | grep -v node_modules`

- [ ] **Step 2: Fix useApiData.ts default**

```typescript
// BEFORE: default pageSize = 20
// AFTER: default pageSize = 10
```

- [ ] **Step 3: Fix api.ts listSessions**

```typescript
// BEFORE: pageSize = 20
// AFTER: pageSize = 10
```

- [ ] **Step 4: Fix sessionSlice.ts**

```typescript
// BEFORE: pageSize = 50
// AFTER: pageSize = 10
```

- [ ] **Step 5: Fix any other inconsistencies found in audit**

- [ ] **Step 6: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind/web && npm run build 2>&1 | tail -10`

- [ ] **Step 7: Commit**

```bash
git add web/src/
git commit -m "fix: standardize pagination default to 10 across all pages"
```

---

### Task A2: Fix Inconsistent Loading States

**Files:**
- Modify: `web/src/components/messages/MessageChannelsTab.tsx:306`
- Modify: `web/src/components/messages/MessagesTab.tsx:247`

**Context:** These pages use `<Loader2>` spinner for page-level loading instead of `<LoadingState variant="page" />`.

- [ ] **Step 1: Audit all loading patterns**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind/web && grep -rn 'Loader2\|<LoadingState\|<Spinner' src/components/ src/pages/ | grep -v node_modules | grep -v '.test.'`

- [ ] **Step 2: Fix MessageChannelsTab**

```typescript
// BEFORE
{loading && <div className="flex justify-center p-8"><Loader2 className="h-6 w-6 animate-spin" /></div>}

// AFTER
{loading ? (
  <LoadingState variant="page" />
) : (
  // ... content
)}
```

- [ ] **Step 3: Fix MessagesTab**

Same pattern as above.

- [ ] **Step 4: Fix any other pages using spinner for page-level loading**

- [ ] **Step 5: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind/web && npm run build 2>&1 | tail -10`

- [ ] **Step 6: Commit**

```bash
git add web/src/
git commit -m "fix: replace page-level spinners with skeleton loading states"
```

---

## Track B: Page-Specific Improvements

### Task B1: Extend Empty State Guidance Components

**Files:**
- Modify: `web/src/components/shared/EmptyState.tsx` (ALREADY EXISTS — do not recreate)
- Modify: relevant page components

**Context:** `EmptyState.tsx` already exists. This task audits it and ensures it's used consistently across all list pages with appropriate guidance.

- [ ] **Step 1: Read existing EmptyState component**

Read `web/src/components/shared/EmptyState.tsx` to understand its current API and capabilities.

- [ ] **Step 2: Extend if needed**

If the existing component lacks action buttons or icon support, extend it:

```typescript
interface EmptyStateProps {
  icon: React.ReactNode;
  title: string;
  description: string;
  action?: {
    label: string;
    onClick: () => void;
  };
}

export function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-16 text-center">
      <div className="mb-4 text-muted-foreground">{icon}</div>
      <h3 className="text-lg font-medium">{title}</h3>
      <p className="mt-1 text-sm text-muted-foreground">{description}</p>
      {action && (
        <Button onClick={action.onClick} className="mt-4">
          {action.label}
        </Button>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Add empty states to key pages**

- Devices page: "No devices connected. Add your first device to start monitoring."
- Agents page: "No agents created. Create an agent to automate your workflows."
- Rules page: "No rules defined. Create a rule to set up automated responses."
- Messages page: "No messages yet."
- Data Explorer: "No data sources available."
- Dashboard: "Add components to your dashboard."

- [ ] **Step 3: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind/web && npm run build 2>&1 | tail -10`

- [ ] **Step 4: Commit**

```bash
git add web/src/
git commit -m "feat: add EmptyState component and integrate across all pages"
```

---

### Task B2: Add Confirmation Dialogs for Dangerous Operations

**Files:**
- Create: `web/src/components/shared/ConfirmDialog.tsx`
- Modify: device delete, agent delete, rule delete, extension uninstall pages

- [ ] **Step 1: Create reusable ConfirmDialog component**

```typescript
interface ConfirmDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: "default" | "destructive";
  onConfirm: () => void;
}
```

- [ ] **Step 2: Add confirmation to delete operations**

Wrap all delete/remove handlers with confirmation dialog.

- [ ] **Step 3: Add confirmation to restart/reset operations**

- [ ] **Step 4: Build and test**

- [ ] **Step 5: Commit**

```bash
git commit -m "feat: add ConfirmDialog and integrate with dangerous operations"
```

---

### Task B3: Add Form Validation with Inline Error Messages

**Files:**
- Modify: agent editor form, device registration form, rule creation form, extension upload form

- [ ] **Step 1: Create form validation utility**

```typescript
// web/src/lib/form-validation.ts
export function validateRequired(value: string, field: string): string | null {
  return value.trim() ? null : `${field} is required`;
}

export function validateLength(value: string, field: string, min: number, max: number): string | null {
  const len = value.trim().length;
  if (len < min) return `${field} must be at least ${min} characters`;
  if (len > max) return `${field} must be at most ${max} characters`;
  return null;
}
```

- [ ] **Step 2: Apply to agent editor form**

Add real-time validation on blur for: name, system prompt, model selection.

- [ ] **Step 3: Apply to device registration form**

- [ ] **Step 4: Apply to rule creation form**

- [ ] **Step 5: Build and test**

- [ ] **Step 6: Commit**

```bash
git commit -m "feat: add inline form validation to agent, device, and rule editors"
```

---

## Track C: Universal UX Patterns

### Task C1: Add Error Boundary Components

**Files:**
- Create: `web/src/components/shared/ErrorBoundary.tsx`
- Modify: `web/src/App.tsx` or root layout

- [ ] **Step 1: Create ErrorBoundary component**

```typescript
export class ErrorBoundary extends React.Component<
  { children: React.ReactNode; fallback?: React.ReactNode },
  { hasError: boolean; error?: Error }
> {
  state = { hasError: false, error: undefined };

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("ErrorBoundary caught:", error, info);
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback || (
        <div className="flex flex-col items-center justify-center py-16">
          <h2 className="text-lg font-medium">Something went wrong</h2>
          <p className="mt-2 text-sm text-muted-foreground">{this.state.error?.message}</p>
          <Button onClick={() => this.setState({ hasError: false })} className="mt-4">
            Try again
          </Button>
        </div>
      );
    }
    return this.props.children;
  }
}
```

- [ ] **Step 2: Wrap page routes with ErrorBoundary**

- [ ] **Step 3: Build and test**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: add ErrorBoundary component for graceful failure handling"
```

---

### Task C2: Improve Error Toast Messages

**Files:**
- Modify: API error handling in `web/src/lib/api.ts`
- Modify: page-level error handlers

- [ ] **Step 1: Create user-friendly error message mapper**

```typescript
export function getUserFriendlyError(error: unknown): string {
  if (error instanceof ApiError) {
    switch (error.status) {
      case 400: return "Invalid request. Please check your input.";
      case 401: return "Session expired. Please refresh the page.";
      case 404: return "The requested resource was not found.";
      case 500: return "Server error. Please try again later.";
      case 502: return "Service temporarily unavailable.";
      case 503: return "Service is starting up. Please wait a moment.";
      default: return error.message || "An unexpected error occurred.";
    }
  }
  if (error instanceof TypeError && error.message.includes("fetch")) {
    return "Unable to connect to the server. Is it running?";
  }
  return "An unexpected error occurred.";
}
```

- [ ] **Step 2: Apply to all catch blocks currently showing raw error messages**

- [ ] **Step 3: Build and test**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: add user-friendly error messages for API failures"
```

---

## Completion Checklist

- [ ] All pages use pagination default of 10
- [ ] All page-level loading uses skeleton, not spinner
- [ ] Zero `alert()` calls remain
- [ ] Empty states with guidance on all list pages
- [ ] Confirmation dialogs on all destructive operations
- [ ] Inline form validation on key forms
- [ ] ErrorBoundary wrapping page routes
- [ ] User-friendly error toast messages
- [ ] `npm run build` passes with no errors
