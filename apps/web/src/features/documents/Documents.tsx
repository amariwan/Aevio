import { useEffect, useState } from "react";
import { FileText, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/hooks/useI18n";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PageHeader } from "@/components/layout/Page";
import { Input } from "@/components/ui/input";
import { AttachmentPicker, CommentComposer, MentionHint } from "@/features/collaboration";
import {
  createAttachment,
  createComment,
  createDocument,
  listAttachments,
  listComments,
  listDocuments,
  updateDocument,
} from "@/lib/api";
import type { WorkspaceAttachment, WorkspaceComment, WorkspaceDocument } from "@/lib/types";

export function Documents() {
  const { t } = useI18n();
  const [documents, setDocuments] = useState<WorkspaceDocument[]>([]);
  const [selectedId, setSelectedId] = useState<string>();
  const [comments, setComments] = useState<WorkspaceComment[]>([]);
  const [attachments, setAttachments] = useState<WorkspaceAttachment[]>([]);
  const selected = documents.find((document) => document.id === selectedId);
  useEffect(() => {
    void listDocuments().then((items) => {
      setDocuments(items);
      setSelectedId(items[0]?.id);
    });
  }, []);
  useEffect(() => {
    if (!selected) return;
    void listComments("document", selected.id).then(setComments);
    void listAttachments("document", selected.id).then(setAttachments);
  }, [selected]);
  const create = async () => {
    const document = await createDocument({ title: "Untitled document", content: "" });
    setDocuments((current) => [document, ...current]);
    setSelectedId(document.id);
  };
  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-canvas text-fg">
      <PageHeader title="Documents">
        <AttachmentPicker
          onAttach={(name) => {
            if (!selected) return;
            void createAttachment({
              targetType: "document",
              targetId: selected.id,
              name,
              path: name,
            }).then((attachment) => setAttachments((current) => [attachment, ...current]));
          }}
        />
        <Button size="sm" className="gap-1.5" onClick={() => void create()}>
          <Plus className="size-3.5" /> New document
        </Button>
      </PageHeader>
      <main className="min-h-0 flex-1 overflow-y-auto p-5">
        <div className="mx-auto grid max-w-6xl gap-4 xl:grid-cols-[minmax(0,1fr)_280px]">
          <Card className="min-h-[560px] border-line bg-panel">
            <CardHeader className="border-b border-line">
              <div className="flex items-center gap-2">
                <FileText className="size-4 text-accent" />
                <Input
                  aria-label="Document title"
                  value={selected?.title ?? ""}
                  onChange={(event) =>
                    setDocuments((current) =>
                      current.map((item) =>
                        item.id === selected?.id ? { ...item, title: event.target.value } : item,
                      ),
                    )
                  }
                  onBlur={(event) =>
                    selected && void updateDocument(selected.id, { title: event.target.value })
                  }
                  className="border-0 bg-transparent px-0 text-base font-semibold text-fg-strong shadow-none focus-visible:ring-0"
                />
              </div>
              <p className="text-xs text-fg-muted">
                Saved locally · document content is persisted in the Aevio database.
              </p>
            </CardHeader>
            <CardContent className="p-0">
              <textarea
                aria-label="Document editor"
                value={selected?.content ?? ""}
                onChange={(event) =>
                  setDocuments((current) =>
                    current.map((item) =>
                      item.id === selected?.id ? { ...item, content: event.target.value } : item,
                    ),
                  )
                }
                onBlur={(event) =>
                  selected && void updateDocument(selected.id, { content: event.target.value })
                }
                className="min-h-[460px] w-full resize-none bg-transparent p-6 font-mono text-sm leading-7 text-fg-strong outline-none placeholder:text-fg-faint"
              />
              <div className="border-t border-line px-6 py-3 text-[11px] text-fg-muted">
                Markdown · @ mentions · attachments
              </div>
            </CardContent>
          </Card>
          <Card className="border-line bg-panel">
            <CardHeader>
              <CardTitle className="text-sm text-fg-strong">{t("documents.comments")}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="rounded-lg border border-dashed border-line p-3 text-xs text-fg-muted">
                <p>Discuss this document and mention a teammate.</p>
                <MentionHint />
              </div>
              {comments.map((entry) => (
                <div key={entry.id} className="border-b border-line pb-3">
                  <p className="text-xs font-semibold text-fg-strong">{entry.author}</p>
                  <p className="mt-1 text-xs leading-relaxed text-fg-muted">{entry.body}</p>
                </div>
              ))}
              <CommentComposer
                onSubmit={(body) =>
                  selected &&
                  void createComment({ targetType: "document", targetId: selected.id, body }).then(
                    (comment) => setComments((current) => [...current, comment]),
                  )
                }
              />
              {attachments.length > 0 && (
                <div className="border-t border-line pt-3">
                  <p className="mb-2 text-xs font-semibold text-fg-strong">
                    {t("documents.attachments")}
                  </p>
                  {attachments.map((attachment) => (
                    <p key={attachment.id} className="truncate text-xs text-fg-muted">
                      {attachment.name}
                    </p>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </main>
    </div>
  );
}
