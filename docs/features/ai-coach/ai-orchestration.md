# Rules and optional AI

Typed `AssistRequest` is validated in Rust before provider access. The deterministic adapter works without AI, filesystem actions or network access. Its source is always `rules`. Optional macOS Foundation Models generation is exposed through a shared Rust boundary with explicit unsupported Windows/Linux adapters and a visible rules fallback.

Prompt version `coach-1` uses sidecar protocol 2; Rust and Swift versions advance together. The provider produces a constrained object of small action steps and a next action. Rust bounds titles, input lengths, granularity, step counts and output text. The user can edit/delete/regenerate steps before accepting them. No AI output directly invokes a mutation.

Coach inference runs off the UI thread, one operation at a time, with a minimum ten-second request interval, a 30-second generation timeout, a 700-token response cap, no automatic retries and no accumulating conversation transcript. Each short-lived sidecar is killed/reaped on drop; the existing classification worker keeps its own process. Unsupported models, invalid outputs, timeouts and crashes produce an explicit local fallback, never an `AI active` claim based only on classification availability.

System model selection is the only currently supported generative provider; remote provider credentials, model lists and token-budget controls are not fabricated. Native reminders use deterministic policies rather than AI decisions. Mutations publish a coach invalidation so each view reloads its own selected date.

Implementation references: [Tauri 2.11.5](https://docs.rs/tauri/2.11.5/tauri/), [Rust Result handling](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html), and existing Swift Foundation Models schema generation.

Daily priority planning uses `coach_plan`, a separate constrained sidecar method. At most three existing task identities are suggested; task identities, uniqueness, completion state and summed capacity are validated again in Rust. The model sees the goal and at most 30 task summaries (bounded titles, priority and estimates). Failed or invalid planning uses the local priority/capacity policy. Users may still plan more than three important tasks. Suggestions never add or postpone tasks implicitly.

When Memory is enabled, up to eight explicitly saved/confirmed notes may join a user-requested on-device breakdown. Disabling it stops retrieval for subsequent requests; notes remain inspectable/editable/deletable. The five coaching styles change local guidance and are included in the on-device prompt context.

Focus Guardian observes only a count of actual adjacent app transitions (at least eight in ten minutes), and requires enabled automatic hints plus native-reminder consent, quiet-hour and cooldown checks. It cannot establish whether a switch was distracting. Existing paused break reminders suppress Coach delivery too. A focus session's planned end generates at most one permitted native check-in; there is no forced session termination or mandatory continuation.
