# Security Model

## Identity and sessions

- Passwords are hashed with Argon2 (canonical parameters) in
  `crates/aevio-services/src/auth.rs` (`hash_password` / `verify_password`).
  Every server adapter delegates to these; no adapter picks its own parameters.
- Session tokens are random UUIDs; only their SHA-256 digest is stored
  (`hash_token`). A session is live only when `revoked_at` is `None` and
  `now - created_at <= 30 days` (see `aevio-services::identity::token_is_live`).
  Revocation always wins over TTL.
- Emails are normalized with `normalize_email` (trim + lowercase) and filtered
  in SQL via `lower(email)`, so identity is case-insensitive without loading
  all accounts.

## Roles

Roles in ascending privilege: `viewer < member < admin < owner`.
- `Role::allows(required)` is the single authorization predicate.
- `decide_membership_change` (services) enforces: an actor may only grant
  roles it itself holds; the last owner of a workspace cannot be demoted;
  `admin` cannot grant `owner`.
- Service credentials never become owners: their effective role is derived
  from their explicit permission scope, never from workspace membership.

## Service tokens

- Configured via `AEVIO_SERVER_TOKEN` (>= 16 chars), `AEVIO_SERVER_TOKEN_WORKSPACES`
  (CSV, fallback `AEVIO_SERVER_TOKEN_WORKSPACE`) and `AEVIO_SERVER_TOKEN_PERMISSIONS`
  (default `read,write`).
- An empty workspace list or permission list makes the token unusable at startup
  (fail-closed). A service actor is never a super-user and is limited to its
  configured workspaces and permissions.
- `AEVIO_SERVER_WORKSPACE` (deployment-wide default) additionally restricts
  which workspace a request may target.

## Authorization points

- `aevio_services::identity::authorize` is the single authorization decision;
  adapters translate `AccessDenial` into their transport response.
- `aevio_http::auth` owns bearer extraction and constant-time token comparison.
- Resource scoping happens both in the use case and in repository filters; a
  request cannot reach a workspace it is not a member of.
