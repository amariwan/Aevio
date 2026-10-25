import { useRef, useState } from "react";
import { useI18n } from "@/hooks/useI18n";
import { Paperclip, Send } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export function CommentComposer({ onSubmit }: { onSubmit: (body: string) => void }) {
  const { t } = useI18n();
  const [value, setValue] = useState("");
  const submit = () => {
    const body = value.trim();
    if (!body) return;
    onSubmit(body);
    setValue("");
  };
  return (
    <div className="flex gap-2">
      <Input
        aria-label="Comment"
        placeholder={t("collaboration.writeComment")}
        value={value}
        onChange={(event) => setValue(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") submit();
        }}
      />
      <Button size="icon-sm" onClick={submit} aria-label="Send comment">
        <Send className="size-3.5" />
      </Button>
    </div>
  );
}

export function AttachmentPicker({ onAttach }: { onAttach: (name: string) => void }) {
  const input = useRef<HTMLInputElement>(null);
  return (
    <>
      <input
        ref={input}
        type="file"
        className="hidden"
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.target.value = "";
          if (file) onAttach(file.name);
        }}
      />
      <Button
        variant="outline"
        size="sm"
        className="gap-1.5"
        onClick={() => input.current?.click()}
      >
        <Paperclip className="size-3.5" /> Attach file
      </Button>
    </>
  );
}

export function MentionHint() {
  return (
    <p className="text-[11px] text-fg-muted">
      Use <span className="font-mono text-accent">@name</span> to mention a teammate.
    </p>
  );
}
