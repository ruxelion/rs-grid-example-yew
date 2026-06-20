//! Demo application showcasing rs-grid with Yew 0.23 CSR.

use std::rc::Rc;

use example_common::{
    build_model, class_map::resolve_classes, fmt_cols, fmt_rows,
    layout::LayoutSnapshot,
};
use rs_grid_web::storage;
use rs_grid_yew::{
    theme_from_css_vars, GridCanvas, Locale, ModelSlot, WebGridCanvas,
};
use wasm_bindgen::prelude::*;
use web_sys::{Event, HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

/// localStorage key for the persisted column layout.
const LS_KEY: &str = "rs-grid-basic-layout";

/// Detect the initial language code from the browser, restricted to the
/// languages the demo offers (falls back to English).
fn initial_lang_code() -> String {
    let detected = web_sys::window()
        .and_then(|w| w.navigator().language())
        .unwrap_or_default();
    detected
        .split('-')
        .next()
        .unwrap_or("en")
        .to_string()
}

/// Apply a CSS class to the document root element (theme switch).
fn set_root_class(cls: &str) {
    if let Some(root) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    {
        root.set_class_name(cls);
    }
}

// ── App component ──────────────────────────────────────────────────────────

#[function_component]
fn App() -> Html {
    // ── Reactive state ────────────────────────────────────────────────────
    let row_count = use_state(|| 1_000u64);
    let col_count = use_state(|| 20usize);
    let theme_class = use_state(String::new);
    let editable = use_state(|| true);
    let selectable = use_state(|| true);
    let column_reorderable = use_state(|| true);
    let lang_code = use_state(initial_lang_code);
    let locale = use_state(Locale::from_browser);
    let validation_error = use_state(String::new);
    let last_button_action = use_state(String::new);

    // Live canvas handle — used by the toggle effects to call `set_*`.
    let grid_ref = use_mut_ref(|| None::<WebGridCanvas>);

    // ── Effect: theme class → root CSS + repaint the grid in the new theme ─
    {
        let grid_ref = grid_ref.clone();
        use_effect_with((*theme_class).clone(), move |cls| {
            set_root_class(cls);
            if let Some(gc) = grid_ref.borrow().as_ref() {
                gc.set_theme(theme_from_css_vars());
            }
        });
    }

    // ── Effect: editable toggle → live canvas ────────────────────────────
    {
        let grid_ref = grid_ref.clone();
        use_effect_with(*editable, move |v| {
            if let Some(gc) = grid_ref.borrow().as_ref() {
                gc.set_editable(*v);
            }
        });
    }

    // ── Effect: selectable toggle → live canvas ──────────────────────────
    {
        let grid_ref = grid_ref.clone();
        use_effect_with(*selectable, move |v| {
            if let Some(gc) = grid_ref.borrow().as_ref() {
                gc.set_selectable(*v);
            }
        });
    }

    // ── Effect: column reorder toggle → live canvas ──────────────────────
    {
        let grid_ref = grid_ref.clone();
        use_effect_with(*column_reorderable, move |v| {
            if let Some(gc) = grid_ref.borrow().as_ref() {
                gc.set_column_reorderable(*v);
            }
        });
    }

    // ── Build the model (with persisted layout applied before mount) ──────
    let mut model = build_model(*row_count, *col_count);
    if let Some(raw) = storage::get_item(LS_KEY) {
        if let Some(snapshot) = LayoutSnapshot::from_json(&raw) {
            snapshot.apply(&mut model);
        }
    }

    // Theme is read from CSS vars after the root class is applied by the
    // effect above; the component re-renders on `theme_class` changes.
    let _ = &*theme_class;
    let theme = theme_from_css_vars();

    // ── Callbacks ─────────────────────────────────────────────────────────

    let on_rows_change = {
        let row_count = row_count.clone();
        Callback::from(move |e: Event| {
            let v = e
                .target_unchecked_into::<HtmlSelectElement>()
                .value()
                .parse::<u64>()
                .unwrap_or(1_000);
            row_count.set(v);
        })
    };

    let on_cols_change = {
        let col_count = col_count.clone();
        Callback::from(move |e: Event| {
            let v = e
                .target_unchecked_into::<HtmlSelectElement>()
                .value()
                .parse::<usize>()
                .unwrap_or(20);
            col_count.set(v);
        })
    };

    let on_theme_change = {
        let theme_class = theme_class.clone();
        Callback::from(move |e: Event| {
            let v = e.target_unchecked_into::<HtmlSelectElement>().value();
            theme_class.set(v);
        })
    };

    let on_lang_change = {
        let lang_code = lang_code.clone();
        let locale = locale.clone();
        Callback::from(move |e: Event| {
            let v = e.target_unchecked_into::<HtmlSelectElement>().value();
            locale.set(Locale::from_language_tag(&v));
            lang_code.set(v);
        })
    };

    let on_editable_change = {
        let editable = editable.clone();
        Callback::from(move |e: Event| {
            let checked =
                e.target_unchecked_into::<HtmlInputElement>().checked();
            editable.set(checked);
        })
    };

    let on_selectable_change = {
        let selectable = selectable.clone();
        Callback::from(move |e: Event| {
            let checked =
                e.target_unchecked_into::<HtmlInputElement>().checked();
            selectable.set(checked);
        })
    };

    let on_reorder_change = {
        let column_reorderable = column_reorderable.clone();
        Callback::from(move |e: Event| {
            let checked =
                e.target_unchecked_into::<HtmlInputElement>().checked();
            column_reorderable.set(checked);
        })
    };

    let on_reset = Callback::from(move |_: MouseEvent| {
        storage::remove_item(LS_KEY);
        if let Some(w) = web_sys::window() {
            let _ = w.location().reload();
        }
    });

    // ── on_mount: wire resolvers / toggles / persistence / button click ──
    let on_mount = {
        let grid_ref = grid_ref.clone();
        let editable = editable.clone();
        let selectable = selectable.clone();
        let column_reorderable = column_reorderable.clone();
        let last_button_action = last_button_action.clone();
        Callback::from(move |gc: WebGridCanvas| {
            gc.set_class_resolver(Rc::new(resolve_classes));
            gc.set_editable(*editable);
            gc.set_selectable(*selectable);
            gc.set_column_reorderable(*column_reorderable);

            // Persist column layout to localStorage so user-resized /
            // reordered columns survive a page reload (F5).
            let gc_save = gc.clone();
            gc.set_on_columns_changed(move || {
                let snapshot = LayoutSnapshot::new(
                    gc_save.column_widths(),
                    gc_save.column_order(),
                    gc_save.pinned_count(),
                );
                if let Some(json) = snapshot.to_json() {
                    storage::set_item(LS_KEY, &json);
                }
            });

            // Cell button clicks → status line.
            let last_button_action = last_button_action.clone();
            gc.set_on_cell_button_click(move |row, col, btn| {
                last_button_action
                    .set(format!("[{btn}] row={row} col={col}"));
            });

            *grid_ref.borrow_mut() = Some(gc);
        })
    };

    // ── on_validation_error: show "[{col}] {msg}" ────────────────────────
    let on_validation_error: rs_grid_yew::ValidationErrorCb = {
        let validation_error = validation_error.clone();
        Rc::new(move |_row: u64, col: String, msg: String| {
            validation_error.set(format!("[{col}] {msg}"));
        })
    };

    // ── View ──────────────────────────────────────────────────────────────

    html! {
        <main class="app-layout">
            <div class="app-page-header">
                <h1 class="app-title">{"rs-grid basic example"}</h1>
                <p class="app-subtitle">
                    {"Use the "}
                    <strong class="app-highlight">
                        { fmt_rows(*row_count) }
                    </strong>
                    {" × "}
                    <strong class="app-highlight">
                        { fmt_cols(*col_count) }
                    </strong>
                    {" virtual dataset below to test windowed rendering."}
                </p>
                <div class="app-controls">

                    // ── Dataset size ──────────────────────────────────
                    <div class="app-control">
                        <span class="app-control-label">{"Dataset size"}</span>
                        <select class="app-control-select"
                            onchange={on_rows_change}>
                            <option value="1000"
                                selected={*row_count == 1_000}>
                                {"1 000 rows"}
                            </option>
                            <option value="100000"
                                selected={*row_count == 100_000}>
                                {"100 000 rows"}
                            </option>
                            <option value="1000000"
                                selected={*row_count == 1_000_000}>
                                {"1 million rows"}
                            </option>
                            <option value="100000000"
                                selected={*row_count == 100_000_000}>
                                {"100 million rows"}
                            </option>
                            <option value="1000000000"
                                selected={*row_count == 1_000_000_000}>
                                {"1 billion rows"}
                            </option>
                            <option value="1000000000000"
                                selected={*row_count == 1_000_000_000_000}>
                                {"1 trillion rows"}
                            </option>
                            <option value="1000000000000000"
                                selected={*row_count
                                    == 1_000_000_000_000_000}>
                                {"1 quadrillion rows"}
                            </option>
                        </select>
                    </div>

                    // ── Column count ──────────────────────────────────
                    <div class="app-control">
                        <span class="app-control-label">
                            {"Column count"}
                        </span>
                        <select class="app-control-select"
                            onchange={on_cols_change}>
                            <option value="20"
                                selected={*col_count == 20}>
                                {"20 columns"}
                            </option>
                            <option value="100"
                                selected={*col_count == 100}>
                                {"100 columns"}
                            </option>
                            <option value="1000"
                                selected={*col_count == 1000}>
                                {"1 000 columns"}
                            </option>
                        </select>
                    </div>

                    // ── Theme ─────────────────────────────────────────
                    <div class="app-control">
                        <span class="app-control-label">{"Theme"}</span>
                        <select class="app-control-select"
                            onchange={on_theme_change}>
                            <option value=""
                                selected={(*theme_class).is_empty()}>
                                {"Light"}
                            </option>
                            <option value="dark"
                                selected={*theme_class == "dark"}>
                                {"Dark"}
                            </option>
                            <option value="dimmed"
                                selected={*theme_class == "dimmed"}>
                                {"Dimmed"}
                            </option>
                        </select>
                    </div>

                    // ── Language ──────────────────────────────────────
                    <div class="app-control">
                        <span class="app-control-label">{"Language"}</span>
                        <select class="app-control-select"
                            value={(*lang_code).clone()}
                            onchange={on_lang_change}>
                            <option value="en"
                                selected={*lang_code == "en"}>
                                {"English"}
                            </option>
                            <option value="fr"
                                selected={*lang_code == "fr"}>
                                {"Fran\u{e7}ais"}
                            </option>
                            <option value="de"
                                selected={*lang_code == "de"}>
                                {"Deutsch"}
                            </option>
                            <option value="es"
                                selected={*lang_code == "es"}>
                                {"Espa\u{f1}ol"}
                            </option>
                            <option value="it"
                                selected={*lang_code == "it"}>
                                {"Italiano"}
                            </option>
                            <option value="pt"
                                selected={*lang_code == "pt"}>
                                {"Portugu\u{ea}s"}
                            </option>
                            <option value="nl"
                                selected={*lang_code == "nl"}>
                                {"Nederlands"}
                            </option>
                            <option value="pl"
                                selected={*lang_code == "pl"}>
                                {"Polski"}
                            </option>
                            <option value="tr"
                                selected={*lang_code == "tr"}>
                                {"T\u{fc}rk\u{e7}e"}
                            </option>
                            <option value="ru"
                                selected={*lang_code == "ru"}>
                                {"Русский"}
                            </option>
                            <option value="uk"
                                selected={*lang_code == "uk"}>
                                {"Українська"}
                            </option>
                            <option value="ar"
                                selected={*lang_code == "ar"}>
                                {"العربية"}
                            </option>
                            <option value="ja"
                                selected={*lang_code == "ja"}>
                                {"日本語"}
                            </option>
                            <option value="zh"
                                selected={*lang_code == "zh"}>
                                {"中文"}
                            </option>
                            <option value="ko"
                                selected={*lang_code == "ko"}>
                                {"한국어"}
                            </option>
                        </select>
                    </div>

                    // ── Editable toggle ───────────────────────────────
                    <div class="app-control">
                        <span class="app-control-label">{"Editable"}</span>
                        <label class="app-switch">
                            <input
                                type="checkbox"
                                checked={*editable}
                                onchange={on_editable_change}
                            />
                            <span class="app-switch-track"></span>
                        </label>
                    </div>

                    // ── Selectable toggle ─────────────────────────────
                    <div class="app-control">
                        <span class="app-control-label">{"Selectable"}</span>
                        <label class="app-switch">
                            <input
                                type="checkbox"
                                checked={*selectable}
                                onchange={on_selectable_change}
                            />
                            <span class="app-switch-track"></span>
                        </label>
                    </div>

                    // ── Column reorder toggle ─────────────────────────
                    <div class="app-control">
                        <span class="app-control-label">
                            {"Column reorder"}
                        </span>
                        <label class="app-switch">
                            <input
                                type="checkbox"
                                checked={*column_reorderable}
                                onchange={on_reorder_change}
                            />
                            <span class="app-switch-track"></span>
                        </label>
                    </div>

                    // ── Reset persisted layout ────────────────────────
                    <div class="app-control">
                        <span class="app-control-label">{"Layout"}</span>
                        <button class="app-control-button"
                            onclick={on_reset}>
                            {"Reset"}
                        </button>
                    </div>
                </div>
            </div>

            // ── Validation error display ──────────────────────────────
            if !(*validation_error).is_empty() {
                <div class="app-validation-error">
                    { (*validation_error).clone() }
                </div>
            }

            // ── Cell button click display ─────────────────────────────
            if !(*last_button_action).is_empty() {
                <div class="app-validation-error">
                    {"Button clicked: "}{ (*last_button_action).clone() }
                </div>
            }

            // ── Body: grid canvas ─────────────────────────────────────
            <div class="app-body">
                <div class="app-grid-wrapper">
                    <GridCanvas
                        key={format!("{}-{}", *row_count, *col_count)}
                        model={ModelSlot::new(model)}
                        width={AttrValue::from("100%")}
                        height={AttrValue::from("100%")}
                        theme={Some(theme)}
                        locale={Some((*locale).clone())}
                        on_mount={on_mount}
                        on_validation_error={on_validation_error}
                    />
                </div>
            </div>
        </main>
    }
}

// ── WASM entry point ───────────────────────────────────────────────────────

/// WASM entry point — mount the Yew app.
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}
