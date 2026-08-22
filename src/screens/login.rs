//! Account — sign in, register, sign out. Doubles as the account screen, which
//! is why signing out and the admin link live here rather than in the chrome.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::auth::{Auth, AuthUser, MIN_PASSWORD_LEN};
use crate::ui::Kicker;

#[component]
pub fn LoginScreen() -> impl IntoView {
    let auth = Auth::from_context();

    view! {
        <Suspense fallback=|| {
            view! { <div class="px-[18px] py-12 text-[13px] text-ink/55">"…"</div> }
        }>
            {move || match auth.user() {
                Some(user) => view! { <SignedIn user /> }.into_any(),
                None => view! { <SignedOut /> }.into_any(),
            }}
        </Suspense>
    }
}

#[component]
fn SignedIn(user: AuthUser) -> impl IntoView {
    let auth = Auth::from_context();
    let is_admin = user.is_admin();

    view! {
        <div>
            <div class="border-b-2 border-ink/40 px-[18px] pt-[18px] pb-4">
                <Kicker class="text-accent-700">"Account"</Kicker>
                <h2 class="mt-2 text-[30px] leading-[1.05] tracking-[-0.025em]">
                    {user.name().to_string()}
                </h2>
                <div class="mt-2.5 text-[12.5px] text-ink/60">{user.email.clone()}</div>
                <div class="mt-1 text-[11px] font-semibold uppercase tracking-[0.14em] text-ink/50">
                    {user.role.label()}
                </div>
            </div>

            <div class="flex flex-col gap-2.5 px-[18px] py-[18px]">
                <Show when=move || is_admin>
                    <A
                        href="/admin"
                        {..}
                        class="btn btn-primary w-full justify-between px-4 py-3.5 no-underline"
                    >
                        "Admin console"
                        <span class="text-[11px] tracking-[0.14em]">"IMPORT"</span>
                    </A>
                </Show>
                <ActionForm action=auth.sign_out>
                    <button
                        type="submit"
                        class="btn btn-secondary w-full justify-start px-4 py-3.5"
                    >
                        "Sign out"
                    </button>
                </ActionForm>
            </div>
        </div>
    }
}

#[component]
fn SignedOut() -> impl IntoView {
    let auth = Auth::from_context();

    view! {
        <div>
            <div class="border-b-2 border-ink/40 px-[18px] pt-[18px] pb-4">
                <Kicker class="text-accent-700">"Account"</Kicker>
                <h2 class="mt-2 text-[30px] leading-[1.05] tracking-[-0.025em]">"Sign in"</h2>
                <p class="mt-2.5 text-[13px] leading-[1.6] text-ink/62 text-pretty">
                    "Your bag lives in this browser either way — an account is for orders, and for the buying desk."
                </p>
            </div>

            <div class="border-b-2 border-ink/40 px-[18px] py-[18px]">
                <ActionForm action=auth.sign_in>
                    <Field label="Email" name="email" input_type="email" autocomplete="email" />
                    <Field
                        label="Password"
                        name="password"
                        input_type="password"
                        autocomplete="current-password"
                    />
                    <FormError message=move || error_of(auth.sign_in.value().get()) />
                    <button
                        type="submit"
                        class="btn btn-primary mt-3.5 w-full justify-between px-4 py-3.5"
                        disabled=move || auth.sign_in.pending().get()
                    >
                        "Sign in"
                        <span class="text-[11px] tracking-[0.14em]">
                            {move || if auth.sign_in.pending().get() { "…" } else { "GO" }}
                        </span>
                    </button>
                </ActionForm>
            </div>

            <div class="px-[18px] py-[18px]">
                <Kicker>"New here"</Kicker>
                <ActionForm action=auth.sign_up>
                    <div class="mt-3">
                        <Field
                            label="Name"
                            name="display_name"
                            input_type="text"
                            autocomplete="name"
                        />
                        <Field label="Email" name="email" input_type="email" autocomplete="email" />
                        <Field
                            label="Password"
                            name="password"
                            input_type="password"
                            autocomplete="new-password"
                            hint=format!("{MIN_PASSWORD_LEN} characters or more")
                        />
                    </div>
                    <FormError message=move || error_of(auth.sign_up.value().get()) />
                    <button
                        type="submit"
                        class="btn btn-secondary mt-3.5 w-full justify-start px-4 py-3.5"
                        disabled=move || auth.sign_up.pending().get()
                    >
                        "Create an account"
                    </button>
                </ActionForm>
            </div>
        </div>
    }
}

/// The action's error, if the last run failed.
fn error_of(value: Option<Result<(), ServerFnError>>) -> Option<String> {
    match value {
        Some(Err(e)) => Some(e.to_string()),
        _ => None,
    }
}

#[component]
pub fn FormError(message: impl Fn() -> Option<String> + Send + Sync + 'static) -> impl IntoView {
    view! {
        {move || {
            message()
                .map(|message| {
                    view! {
                        <p
                            role="alert"
                            class="mt-3 border-l-2 border-accent bg-accent-100 px-3 py-2 text-[12.5px] leading-[1.5] text-accent-800"
                        >
                            {message}
                        </p>
                    }
                })
        }}
    }
}

#[component]
fn Field(
    label: &'static str,
    name: &'static str,
    #[prop(default = "text")] input_type: &'static str,
    #[prop(default = "off")] autocomplete: &'static str,
    #[prop(optional, into)] hint: String,
) -> impl IntoView {
    let has_hint = !hint.is_empty();

    view! {
        <label class="mt-3 block first:mt-0">
            <span class="mb-1.5 block text-[11px] font-semibold uppercase tracking-[0.14em] text-ink/60">
                {label}
            </span>
            <input
                type=input_type
                name=name
                autocomplete=autocomplete
                class="w-full border-2 border-ink bg-transparent px-3 py-[11px] text-[14.5px] font-semibold text-ink caret-accent outline-none focus-visible:border-accent"
            />
            <Show when=move || has_hint>
                <span class="mt-1 block text-[11px] text-ink/50">{hint.clone()}</span>
            </Show>
        </label>
    }
}
