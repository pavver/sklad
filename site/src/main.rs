//! The website of Sklad: one page per language, composed only from Operator UI components.
//! The language comes from the `lang` attribute of the page that loads the program.

mod text;

use leptos::prelude::*;
use operator_ui::prelude::*;
use operator_ui::tokens::icons;
use text::{ADD_REPOSITORY, DESIGN_SYSTEM, EN, FROM_SOURCE, INSTALL, PAGES, REPOSITORY, TRUST_KEY, Text, UK};

/// The texts for the language of the page.
fn page_text() -> &'static Text {
    let lang = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
        .and_then(|e| e.get_attribute("lang"));
    match lang.as_deref() {
        Some("uk") => &UK,
        _ => &EN,
    }
}

/// Whether the system prefers the dark theme; dark when it cannot tell.
fn prefers_dark() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: light)").ok().flatten())
        .is_none_or(|m| !m.matches())
}

/// Opens the page of the language chosen by its name.
fn open_page(name: String) {
    let page = PAGES
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, page)| *page);
    if let (Some(page), Some(window)) = (page, web_sys::window())
        && page_text().language_name != name
    {
        let _ = window.location().set_href(page);
    }
}

#[component]
fn Command(caption: &'static str, code: &'static str) -> impl IntoView {
    view! {
        <div class="site-step">
            <p class="op-t-body">{caption}</p>
            <CodeBlock code=code.to_string() language="sh".to_string()/>
        </div>
    }
}

#[component]
fn Page() -> impl IntoView {
    let text = page_text();
    let dark = RwSignal::new(prefers_dark());
    let language = RwSignal::new(text.language_name.to_owned());
    let features = text
        .features
        .iter()
        .map(|f| {
            view! {
                <ListItem
                    title=f.title
                    subtitle=f.text
                    subtitle_max_lines=3
                    leading=ViewFn::from(|| view! { <Icon icon=icons::CHECK_CIRCLE/> })
                />
            }
        })
        .collect_view();
    let questions = text
        .questions
        .iter()
        .map(|q| {
            view! {
                <Collapsible title=q.title>
                    <p class="op-t-body site-answer">{q.answer}</p>
                </Collapsible>
            }
        })
        .collect_view();
    let actions = ViewFn::from(move || {
        view! {
            <div class="site-language">
                <Combobox
                    options=PAGES.iter().map(|(name, _)| (*name).to_owned()).collect()
                    selected=language
                    on_select=Callback::new(open_page)
                />
            </div>
            <ThemeToggle
                dark=dark
                on_toggle=Callback::new(move |v: bool| dark.set(v))
                width=13.0
                height=6.0
                label=text.dark_mode
            />
        }
    });

    view! {
        <ThemeRoot dark=dark class="site-root">
            <Header title=text.name subtitle=text.tagline actions=actions/>
            <main class="site-main">
                <section class="site-hero">
                    <h1 class="op-t-screen-title">{text.headline}</h1>
                    <p class="op-t-body site-lead">{text.lead}</p>
                    <div class="site-actions">
                        <ActionButton
                            text=text.install_action
                            icon=icons::DOWNLOAD
                            href="#install"
                        />
                        <ActionButton
                            text=text.source_action
                            icon=icons::OPEN_IN_NEW
                            href=REPOSITORY
                            new_tab=true
                        />
                    </div>
                    <img
                        class="site-shot"
                        src=text.catalogue_shot
                        alt=text.catalogue_alt
                        width="1360"
                        height="1040"
                    />
                    <Banner tone=Tone::Warning title=text.early_title text=text.early_text/>
                </section>

                <section class="site-section">
                    <h2 class="op-t-section-title">{text.features_title}</h2>
                    <Card>{features}</Card>
                </section>

                <section class="site-section">
                    <h2 class="op-t-section-title">{text.removal_title}</h2>
                    <img
                        class="site-shot"
                        src=text.removal_shot
                        alt=text.removal_alt
                        width="1360"
                        height="1040"
                        loading="lazy"
                    />
                    <p class="op-t-body">{text.removal_text}</p>
                </section>

                <section class="site-section" id="install">
                    <h2 class="op-t-section-title">{text.install_title}</h2>
                    <Command caption=text.trust_key code=TRUST_KEY/>
                    <Command caption=text.add_repository code=ADD_REPOSITORY/>
                    <Command caption=text.install code=INSTALL/>
                    <Command caption=text.from_source code=FROM_SOURCE/>
                </section>

                <section class="site-section">
                    <h2 class="op-t-section-title">{text.questions_title}</h2>
                    <Accordion>{questions}</Accordion>
                </section>
            </main>
            <Divider/>
            <footer class="site-footer">
                <ActionButton text="GitHub" icon=icons::OPEN_IN_NEW href=REPOSITORY new_tab=true/>
                <ActionButton
                    text=text.design_system
                    icon=icons::OPEN_IN_NEW
                    href=DESIGN_SYSTEM
                    new_tab=true
                />
                <span class="op-t-caption site-muted">{text.license}</span>
            </footer>
        </ThemeRoot>
    }
}

/// The browser looks for the anchor of the address before the page exists, so it is found again here.
fn scroll_to_anchor() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let anchor = window.location().hash().unwrap_or_default();
    let target = anchor
        .strip_prefix('#')
        .filter(|id| !id.is_empty())
        .and_then(|id| window.document()?.get_element_by_id(id));
    if let Some(target) = target {
        target.scroll_into_view();
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(Page);
    scroll_to_anchor();
}
