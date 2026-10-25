import { useEffect, useMemo, useRef, useState } from "react";
import { useI18n } from "@/hooks/useI18n";
import { EditorContent, useEditor } from "@tiptap/react";
import CharacterCount from "@tiptap/extension-character-count";
import Highlight from "@tiptap/extension-highlight";
import Link from "@tiptap/extension-link";
import { TaskItem, TaskList } from "@tiptap/extension-list";
import StarterKit from "@tiptap/starter-kit";
import { TableKit } from "@tiptap/extension-table";
import TextAlign from "@tiptap/extension-text-align";
import Placeholder from "@tiptap/extension-placeholder";
import Typography from "@tiptap/extension-typography";
import Underline from "@tiptap/extension-underline";
import {
  AlignCenter,
  AlignLeft,
  AlignRight,
  Bold,
  Check,
  CheckSquare2,
  ChevronDown,
  Clock3,
  Code2,
  Eraser,
  FileText,
  Folder,
  FolderOpen,
  Highlighter,
  Heading1,
  Heading2,
  Italic,
  Link2,
  List,
  ListOrdered,
  Minus,
  MoreHorizontal,
  Plus,
  Quote,
  Redo2,
  Search,
  StickyNote,
  Table2,
  Trash2,
  Underline as UnderlineIcon,
  Unlink,
  Undo2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { PageHeader } from "@/components/layout/Page";
import { createNote, deleteNote, listNotes, updateNote } from "@/lib/api";
import type { WorkspaceNote } from "@/lib/types";

function escapeHtml(value: string): string {
  return value.replace(
    /[&<>"]/g,
    (character) =>
      ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
      })[character] ?? character,
  );
}

function toEditorContent(body: string): string {
  if (!body.trim()) return "<p></p>";
  if (/<[a-z][\s\S]*>/i.test(body)) return body;
  return body
    .split(/\n{2,}/)
    .map((paragraph) => `<p>${escapeHtml(paragraph).replace(/\n/g, "<br>")}</p>`)
    .join("");
}

function plainText(body: string): string {
  if (!body.includes("<")) return body;
  const container = document.createElement("div");
  container.innerHTML = body;
  return container.textContent ?? "";
}

function formatUpdatedAt(timestamp: number): string {
  return new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(
    new Date(timestamp),
  );
}

type ToolbarButtonProps = {
  label: string;
  active?: boolean;
  onClick: () => void;
  children: React.ReactNode;
};

function ToolbarButton({ label, active = false, onClick, children }: ToolbarButtonProps) {
  return (
    <Button
      type="button"
      variant="ghost"
      size="icon-sm"
      aria-label={label}
      aria-pressed={active}
      title={label}
      className={active ? "bg-accent-soft text-fg-strong" : "text-fg-muted"}
      onMouseDown={(event) => event.preventDefault()}
      onClick={onClick}
    >
      {children}
    </Button>
  );
}

function NoteEditor({
  note,
  onBodyChange,
}: {
  note: WorkspaceNote;
  onBodyChange: (body: string) => void;
}) {
  const { t } = useI18n();
  const [saveState, setSaveState] = useState<"saved" | "saving">("saved");
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const editor = useEditor({
    extensions: [
      StarterKit,
      CharacterCount,
      Highlight.configure({ multicolor: true }),
      Link.configure({ autolink: true, linkOnPaste: true, openOnClick: false }),
      TableKit,
      TaskList,
      TaskItem.configure({ nested: true }),
      TextAlign.configure({ types: ["heading", "paragraph"] }),
      Typography,
      Underline,
      Placeholder.configure({ placeholder: "Capture an idea, decision, or next step…" }),
    ],
    content: toEditorContent(note.body),
    editorProps: { attributes: { class: "tiptap-notes-editor min-h-[360px] outline-none" } },
    onUpdate: ({ editor: nextEditor }) => {
      const body = nextEditor.getHTML();
      onBodyChange(body);
      setSaveState("saving");
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(() => {
        void updateNote(note.id, { body }).then(() => setSaveState("saved"));
      }, 650);
    },
  });

  useEffect(
    () => () => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
    },
    [],
  );

  if (!editor) return <div className="min-h-[360px] animate-pulse rounded-lg bg-surface" />;
  const run = (command: () => boolean) => {
    command();
    editor.commands.focus();
  };
  const editLink = () => {
    const currentHref = editor.getAttributes("link").href as string | undefined;
    const href = window.prompt(t("notes.linkUrl"), currentHref ?? "https://");
    if (href === null) return;
    if (!href.trim()) {
      run(() => editor.chain().unsetLink().run());
      return;
    }
    run(() => editor.chain().setLink({ href: href.trim() }).run());
  };

  return (
    <div className="relative">
      <div className="flex min-h-9 flex-wrap items-center justify-end gap-0.5 border-b border-line/60 pb-2">
        <ToolbarButton
          label="Bold"
          active={editor.isActive("bold")}
          onClick={() => run(() => editor.chain().toggleBold().run())}
        >
          <Bold className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label={t("notes.italic")}
          active={editor.isActive("italic")}
          onClick={() => run(() => editor.chain().toggleItalic().run())}
        >
          <Italic className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label={t("notes.underline")}
          active={editor.isActive("underline")}
          onClick={() => run(() => editor.chain().toggleUnderline().run())}
        >
          <UnderlineIcon className="size-3.5" />
        </ToolbarButton>
        <span className="mx-1 h-5 w-px bg-line" />
        <ToolbarButton
          label="Heading 1"
          active={editor.isActive("heading", { level: 1 })}
          onClick={() => run(() => editor.chain().toggleHeading({ level: 1 }).run())}
        >
          <Heading1 className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label="Heading 2"
          active={editor.isActive("heading", { level: 2 })}
          onClick={() => run(() => editor.chain().toggleHeading({ level: 2 }).run())}
        >
          <Heading2 className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label={t("notes.bulletedList")}
          active={editor.isActive("bulletList")}
          onClick={() => run(() => editor.chain().toggleBulletList().run())}
        >
          <List className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label={t("notes.numberedList")}
          active={editor.isActive("orderedList")}
          onClick={() => run(() => editor.chain().toggleOrderedList().run())}
        >
          <ListOrdered className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label={t("notes.quote")}
          active={editor.isActive("blockquote")}
          onClick={() => run(() => editor.chain().toggleBlockquote().run())}
        >
          <Quote className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label="Code"
          active={editor.isActive("codeBlock")}
          onClick={() => run(() => editor.chain().toggleCodeBlock().run())}
        >
          <Code2 className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label={t("notes.taskList")}
          active={editor.isActive("taskList")}
          onClick={() => run(() => editor.chain().toggleTaskList().run())}
        >
          <CheckSquare2 className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label={t("notes.highlight")}
          active={editor.isActive("highlight")}
          onClick={() => run(() => editor.chain().toggleHighlight().run())}
        >
          <Highlighter className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label={t("notes.addLink")}
          active={editor.isActive("link")}
          onClick={editLink}
        >
          <Link2 className="size-3.5" />
        </ToolbarButton>
        {editor.isActive("link") && (
          <ToolbarButton
            label={t("notes.removeLink")}
            onClick={() => run(() => editor.chain().unsetLink().run())}
          >
            <Unlink className="size-3.5" />
          </ToolbarButton>
        )}
        <ToolbarButton
          label={t("notes.alignLeft")}
          active={editor.isActive({ textAlign: "left" })}
          onClick={() => run(() => editor.chain().setTextAlign("left").run())}
        >
          <AlignLeft className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label={t("notes.alignCenter")}
          active={editor.isActive({ textAlign: "center" })}
          onClick={() => run(() => editor.chain().setTextAlign("center").run())}
        >
          <AlignCenter className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label={t("notes.alignRight")}
          active={editor.isActive({ textAlign: "right" })}
          onClick={() => run(() => editor.chain().setTextAlign("right").run())}
        >
          <AlignRight className="size-3.5" />
        </ToolbarButton>
        <span className="mx-1 h-5 w-px bg-line" />
        <ToolbarButton
          label={t("notes.insertTable")}
          onClick={() =>
            run(() => editor.chain().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run())
          }
        >
          <Table2 className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label={t("notes.horizontalRule")}
          onClick={() => run(() => editor.chain().setHorizontalRule().run())}
        >
          <Minus className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton
          label={t("notes.clearFormatting")}
          onClick={() => run(() => editor.chain().clearNodes().unsetAllMarks().run())}
        >
          <Eraser className="size-3.5" />
        </ToolbarButton>
        <span className="mx-1 h-5 w-px bg-line" />
        <ToolbarButton label="Undo" onClick={() => run(() => editor.chain().undo().run())}>
          <Undo2 className="size-3.5" />
        </ToolbarButton>
        <ToolbarButton label="Redo" onClick={() => run(() => editor.chain().redo().run())}>
          <Redo2 className="size-3.5" />
        </ToolbarButton>
        <span className="ml-auto flex items-center gap-1.5 px-2 text-[11px] text-fg-faint">
          {saveState === "saved" ? (
            <Check className="size-3 text-success" />
          ) : (
            <Clock3 className="size-3" />
          )}
          {saveState === "saved" ? t("notes.savedLocally") : "Saving…"}
        </span>
      </div>
      <EditorContent editor={editor} className="py-8 sm:py-10" />
      <p className="border-t border-line/40 pt-3 text-right text-[10px] text-fg-faint">
        {editor.storage.characterCount.characters().toLocaleString()} characters ·{" "}
        {editor.storage.characterCount.words().toLocaleString()} words
      </p>
    </div>
  );
}

export function Notes() {
  const { t } = useI18n();
  const [notes, setNotes] = useState<WorkspaceNote[]>([]);
  const [selectedId, setSelectedId] = useState<string>();
  const [query, setQuery] = useState("");
  const titleSaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    void listNotes().then((items) => {
      setNotes(items);
      setSelectedId(items[0]?.id);
    });
  }, []);

  const selected = notes.find((note) => note.id === selectedId) ?? notes[0];
  const filteredNotes = useMemo(() => {
    const normalizedQuery = query.trim().toLocaleLowerCase();
    if (!normalizedQuery) return notes;
    return notes.filter((note) =>
      `${note.title} ${plainText(note.body)}`.toLocaleLowerCase().includes(normalizedQuery),
    );
  }, [notes, query]);
  const update = (patch: Partial<WorkspaceNote>) =>
    setNotes((current) =>
      current.map((note) => (note.id === selected?.id ? { ...note, ...patch } : note)),
    );
  const updateTitle = (title: string) => {
    if (!selected) return;
    update({ title });
    if (titleSaveTimer.current) clearTimeout(titleSaveTimer.current);
    titleSaveTimer.current = setTimeout(() => {
      void updateNote(selected.id, { title });
    }, 500);
  };
  const add = async () => {
    const note = await createNote({ title: t("notes.untitled"), body: "" });
    setNotes((current) => [note, ...current]);
    setSelectedId(note.id);
    setQuery("");
  };
  const remove = async () => {
    if (!selected) return;
    await deleteNote(selected.id);
    const remaining = notes.filter((note) => note.id !== selected.id);
    setNotes(remaining);
    setSelectedId(remaining[0]?.id);
  };

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-canvas text-fg">
      <PageHeader title="Notes">
        <Button size="sm" className="gap-1.5" onClick={add}>
          <Plus className="size-3.5" /> New note
        </Button>
      </PageHeader>
      <main className="grid min-h-0 flex-1 grid-cols-[190px_300px_minmax(0,1fr)] overflow-hidden max-[900px]:grid-cols-[190px_minmax(0,1fr)] max-[700px]:grid-cols-[minmax(0,1fr)]">
        <aside className="flex min-h-0 flex-col border-r border-line bg-rail px-3 py-4 max-[900px]:hidden">
          <div className="mb-5 flex items-center justify-between px-2">
            <p className="text-[11px] font-semibold tracking-tight text-fg-muted">
              {t("notes.folders")}
            </p>
            <ChevronDown className="size-3.5 text-fg-faint" />
          </div>
          <button
            type="button"
            className="flex items-center gap-2 rounded-md bg-accent-soft px-2.5 py-2 text-left text-xs font-medium text-fg-strong"
          >
            <FolderOpen className="size-4 text-accent-dim" />
            <span className="truncate">{t("notes.allNotes")}</span>
            <span className="ml-auto text-[10px] text-fg-faint">{notes.length}</span>
          </button>
          <div className="mt-auto border-t border-line pt-3">
            <div className="flex items-center gap-2 px-2 text-[11px] text-fg-faint">
              <Folder className="size-3.5" /> On My Mac
            </div>
            <p className="mt-2 px-2 text-[10px] leading-relaxed text-fg-ghost">
              Your notes are stored locally on this Mac.
            </p>
          </div>
        </aside>
        <aside className="flex min-h-0 flex-col border-r border-line bg-surface/40 max-[700px]:hidden">
          <div className="border-b border-line px-4 py-3">
            <div className="mb-3 flex items-center justify-between">
              <div>
                <p className="text-sm font-semibold text-fg-strong">{t("notes.notes")}</p>
                <p className="mt-0.5 text-[11px] text-fg-faint">
                  {notes.length} {notes.length === 1 ? "note" : "notes"}
                </p>
              </div>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={add}
                aria-label={t("notes.newNote")}
                title={t("notes.newNote")}
              >
                <Plus className="size-4 text-accent-dim" />
              </Button>
            </div>
            <div className="relative">
              <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-fg-faint" />
              <Input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder={t("notes.search")}
                aria-label={t("notes.searchNotes")}
                className="h-8 border-line bg-surface pl-8 text-xs"
              />
            </div>
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto p-2">
            {filteredNotes.map((note) => (
              <button
                key={note.id}
                type="button"
                onClick={() => setSelectedId(note.id)}
                className={`mb-1 w-full rounded-md px-3 py-2.5 text-left transition-colors ${selected?.id === note.id ? "bg-accent-soft text-fg-strong" : "text-fg-muted hover:bg-surface"}`}
              >
                <span className="flex items-center justify-between gap-2">
                  <span className="truncate text-[13px] font-semibold">
                    {note.title || t("notes.untitled")}
                  </span>
                  <span className="shrink-0 text-[10px] text-fg-faint">
                    {new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" }).format(
                      new Date(note.updatedAt),
                    )}
                  </span>
                </span>
                <span className="mt-1 block truncate text-[11px] leading-4 text-fg-faint">
                  {plainText(note.body) || t("notes.noContent")}
                </span>
              </button>
            ))}
            {filteredNotes.length === 0 && (
              <p className="px-2 py-8 text-center text-xs text-fg-faint">No notes found.</p>
            )}
          </div>
        </aside>
        <section className="min-w-0 overflow-y-auto bg-canvas">
          <div className="mx-auto max-w-3xl space-y-5 px-6 py-5 sm:px-12 sm:py-10">
            {selected ? (
              <>
                <div className="flex items-center justify-between gap-4">
                  <span className="flex items-center gap-2 text-[11px] text-fg-faint">
                    <FileText className="size-3.5" /> {formatUpdatedAt(selected.updatedAt)}
                  </span>
                  <div className="flex items-center gap-1">
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      aria-label={t("notes.moreActions")}
                      title={t("notes.moreActions")}
                    >
                      <MoreHorizontal className="size-4 text-fg-muted" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      onClick={remove}
                      aria-label={t("notes.deleteNote")}
                      title={t("notes.deleteNote")}
                    >
                      <Trash2 className="size-4 text-danger" />
                    </Button>
                  </div>
                </div>
                <Input
                  aria-label={t("notes.title")}
                  value={selected.title}
                  placeholder={t("notes.untitled")}
                  onChange={(event) => updateTitle(event.target.value)}
                  onBlur={(event) =>
                    void updateNote(selected.id, {
                      title: event.target.value.trim() || t("notes.untitled"),
                    })
                  }
                  className="border-0 bg-transparent px-0 text-[2.35rem] font-semibold tracking-[-0.05em] text-fg-strong shadow-none placeholder:text-fg-faint focus-visible:ring-0"
                />
                <NoteEditor
                  key={selected.id}
                  note={selected}
                  onBodyChange={(body) => update({ body, updatedAt: Date.now() })}
                />
              </>
            ) : (
              <Card>
                <CardContent className="p-10 text-center">
                  <StickyNote className="mx-auto mb-3 size-8 text-accent-dim" />
                  <p className="text-sm font-medium text-fg-strong">{t("notes.empty")}</p>
                  <p className="mt-1 text-sm text-fg-muted">
                    Create a note to capture the next idea.
                  </p>
                  <Button size="sm" className="mt-5 gap-1.5" onClick={add}>
                    <Plus className="size-3.5" /> New note
                  </Button>
                </CardContent>
              </Card>
            )}
          </div>
        </section>
      </main>
    </div>
  );
}
