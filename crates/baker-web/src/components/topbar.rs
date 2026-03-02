use dioxus::prelude::*;

#[component]
pub fn Topbar() -> Element {
    rsx! {
        header { class: "topbar",
            div { class: "topbar-inner",
                // Brand
                a { class: "brand", href: "#",
                    span { class: "brand-logo",
                        // Baker 'B' icon SVG
                        svg {
                            xmlns: "http://www.w3.org/2000/svg",
                            view_box: "0 0 40 40",
                            fill: "none",
                            width: "32",
                            height: "32",
                            rect { width: "40", height: "40", rx: "8", fill: "#F97316" }
                            path {
                                d: "M10 28V12h9c4 0 6 2 6 5a4 4 0 0 1-2.5 3.8C25 21.5 27 23.3 27 26c0 3.2-2.2 5.3-6.5 5.3H10V28zm4-11h5c1.5 0 2.3-.7 2.3-2s-.8-2-2.3-2h-5v4zm0 8h5.5c1.8 0 2.8-.9 2.8-2.4 0-1.5-1-2.3-2.8-2.3H14v4.7z",
                                fill: "white"
                            }
                        }
                    }
                    span { class: "brand-name", "Baker" }
                    span { class: "brand-version", "v0.14" }
                }

                // Nav links
                nav { class: "topbar-nav",
                    a {
                        href: "https://github.com/aliev/baker",
                        target: "_blank",
                        rel: "noopener",
                        "Docs"
                    }
                    a {
                        href: "https://github.com/aliev/baker/tree/main/examples",
                        target: "_blank",
                        rel: "noopener",
                        "Examples"
                    }
                    a {
                        class: "nav-pill",
                        href: "https://github.com/aliev/baker",
                        target: "_blank",
                        rel: "noopener",
                        svg {
                            xmlns: "http://www.w3.org/2000/svg",
                            view_box: "0 0 16 16",
                            width: "16",
                            height: "16",
                            fill: "currentColor",
                            path {
                                d: "M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0 0 16 8c0-4.42-3.58-8-8-8z"
                            }
                        }
                        " GitHub"
                    }
                }
            }
        }
    }
}
