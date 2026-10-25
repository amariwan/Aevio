import AIDiffReview, {
  type DiffLine,
  type FileChange,
} from "@/components/kokonutui/ai-diff-review";
import type { EntryDetail, Project, Category, SuggestionField } from "@/lib/types";

interface EntrySuggestionDiffReviewProps {
  detail: EntryDetail;
  categories: Category[];
  projects: Project[];
  suggestProjects: boolean;
  onSetField: (field: SuggestionField, valueId: string | null) => void;
}

function valueName(
  field: SuggestionField,
  valueId: string | undefined,
  categories: Category[],
  projects: Project[],
): string {
  if (field === "category") {
    return categories.find((category) => category.id === valueId)?.name ?? "Unknown";
  }
  if (!valueId) return "No project";
  return projects.find((project) => project.id === valueId)?.name ?? "Unknown";
}

function changeLines(before: string, after: string): DiffLine[] {
  return [
    { type: "remove", text: before },
    { type: "add", text: after },
  ];
}

export function EntrySuggestionDiffReview({
  detail,
  categories,
  projects,
  suggestProjects,
  onSetField,
}: EntrySuggestionDiffReviewProps) {
  const files: FileChange[] = detail.suggestions
    .filter(
      (suggestion) =>
        suggestion.outcome !== "rejected" && (suggestion.field === "category" || suggestProjects),
    )
    .flatMap((suggestion) => {
      const beforeId =
        suggestion.field === "category" ? detail.entry.categoryId : detail.entry.projectId;
      const afterId = suggestion.valueId;
      if (beforeId === afterId) return [];
      const before = valueName(suggestion.field, beforeId, categories, projects);
      const after = valueName(suggestion.field, afterId, categories, projects);
      return [
        {
          path: `time-entry/${detail.entry.id}/${suggestion.field}.json`,
          hunks: [
            {
              start: 1,
              lines: changeLines(`${suggestion.field}: ${before}`, `${suggestion.field}: ${after}`),
            },
          ],
        },
      ];
    });

  if (files.length === 0) return null;

  return (
    <section className="rounded-lg border border-line bg-surface p-3">
      <div className="mb-2">
        <h3 className="font-semibold text-fg text-sm">AI changes</h3>
        <p className="text-fg-muted text-xs">Review the suggested category and project updates.</p>
      </div>
      <AIDiffReview
        key={detail.entry.id}
        files={files}
        onDecision={(path, decision) => {
          if (decision !== "accepted") return;
          const field = path.split("/").at(-1)?.replace(".json", "");
          const suggestion = detail.suggestions.find((item) => item.field === field);
          if (suggestion && (field === "category" || field === "project")) {
            onSetField(field, suggestion.valueId ?? null);
          }
        }}
      />
    </section>
  );
}
