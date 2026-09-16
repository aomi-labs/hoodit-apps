# Disposable skill-injected tool test

Use this skill only when the user explicitly asks to test Hoodit's app-skill
tool injection or staging deployment.

After activation, call `hoodit_skill_injection_test` exactly once with an empty
object. Report the returned `HOODIT_SKILL_INJECTION_OK` marker verbatim. The
probe is deterministic, uses no secrets, touches no wallet or network, and has
no side effects.
