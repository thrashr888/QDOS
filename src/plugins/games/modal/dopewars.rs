//! Dope Wars game modal rendering
//!
//! This module handles the visual rendering of the Dope Wars game within
//! the games plugin modal.

use crate::app::ThemeColors;
use crate::plugins::games::dopewars::{DopeWarsState, DopeWarsView, Location, Product};
use crate::ui::components::FullScreenView;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
    Frame,
};

/// Renders the Dope Wars game state to the terminal.
pub fn draw_dopewars(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DopeWarsState,
    colors: &ThemeColors,
) {
    match state.view {
        DopeWarsView::Market => draw_market(frame, view, state, colors),
        DopeWarsView::Travel => draw_travel(frame, view, state, colors),
        DopeWarsView::Status => draw_status(frame, view, state, colors),
        DopeWarsView::Event => draw_event(frame, view, state, colors),
    }
}

fn draw_market(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DopeWarsState,
    colors: &ThemeColors,
) {
    let mut row = 0;

    // Header with day, location, cash, debt
    view.render_row(
        frame,
        row,
        vec![
            Span::styled(
                format!("Day {}/{} ", state.day, 30),
                Style::default().fg(colors.yellow()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  Location: {} ", state.location.name()),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Cash: $", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.cash),
                Style::default().fg(colors.green()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Debt: $", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.debt),
                Style::default().fg(colors.red()),
            ),
            Span::styled("  Space: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}/100", state.inventory.total_items()),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Health: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.health),
                if state.health > 50 {
                    Style::default().fg(colors.green())
                } else if state.health > 25 {
                    Style::default().fg(colors.yellow())
                } else {
                    Style::default().fg(colors.red())
                },
            ),
            Span::styled("  Guns: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.guns),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 1;

    // Market prices header
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "═══════ MARKET ═══════════════════════════════════════════════════",
            Style::default().fg(colors.blue()),
        )],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Product      ", Style::default().fg(colors.grey())),
            Span::styled("Price       ", Style::default().fg(colors.grey())),
            Span::styled("Inventory", Style::default().fg(colors.grey())),
        ],
    );
    row += 1;

    // List products
    for (idx, &product) in Product::all().iter().enumerate() {
        let selected = idx == state.selected_product;
        let cursor = if selected { "▶ " } else { "  " };
        let price_str = match state.market.get_price(product) {
            Some(price) => format!("${:>8}", price),
            None => "  ------".to_string(),
        };

        let inv_qty = state.inventory.get_quantity(product);
        let inv_str = if inv_qty > 0 {
            format!("{:>3}", inv_qty)
        } else {
            "---".to_string()
        };

        let style = if selected {
            Style::default().fg(colors.yellow()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg())
        };

        view.render_row(
            frame,
            row,
            vec![
                Span::styled(cursor, Style::default().fg(colors.red())),
                Span::styled(format!("{:<12} ", product.name()), style),
                Span::styled(price_str, style),
                Span::styled(format!("    {}", inv_str), Style::default().fg(colors.cyan())),
            ],
        );
        row += 1;
    }

    row += 1;

    // Quantity input
    let qty_display = if state.quantity_buffer.is_empty() {
        "1".to_string()
    } else {
        state.quantity_buffer.clone()
    };

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Quantity: ", Style::default().fg(colors.grey())),
            Span::styled(
                qty_display,
                Style::default().fg(colors.green()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (type number)", Style::default().fg(colors.grey())),
        ],
    );
    row += 2;

    // Message
    if let Some(ref msg) = state.message {
        view.render_row(
            frame,
            row,
            vec![Span::styled(msg, Style::default().fg(colors.yellow()))],
        );
    }

    // Help text
    view.render_help(
        frame,
        vec![
            ("↑↓", "select"),
            ("B", "buy"),
            ("S", "sell"),
            ("T", "travel"),
            ("D", "pay debt"),
            ("I", "info"),
            ("P", "pause"),
            ("Esc", "quit"),
        ],
    );
}

fn draw_travel(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DopeWarsState,
    colors: &ThemeColors,
) {
    let mut row = 0;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "═══════ TRAVEL ═══════════════════════════════════════════════════",
            Style::default().fg(colors.blue()),
        )],
    );
    row += 2;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Where do you want to go?",
            Style::default().fg(colors.fg()),
        )],
    );
    row += 2;

    // List locations
    for (idx, &location) in Location::all().iter().enumerate() {
        let selected = idx == state.selected_location;
        let current = location == state.location;
        let cursor = if selected { "▶ " } else { "  " };

        let mut spans = vec![
            Span::styled(cursor, Style::default().fg(colors.red())),
            Span::styled(
                location.name(),
                if selected {
                    Style::default().fg(colors.yellow()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors.fg())
                },
            ),
        ];

        if current {
            spans.push(Span::styled(
                " (current)",
                Style::default().fg(colors.grey()),
            ));
        }

        view.render_row(frame, row, spans);
        row += 1;
    }

    // Help text
    view.render_help(
        frame,
        vec![
            ("↑↓", "select"),
            ("Enter", "travel"),
            ("Esc", "back"),
        ],
    );
}

fn draw_status(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DopeWarsState,
    colors: &ThemeColors,
) {
    let mut row = 0;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "═══════ STATUS ════════════════════════════════════════════════════",
            Style::default().fg(colors.blue()),
        )],
    );
    row += 2;

    // Days remaining
    let days_left = 30 - state.day + 1;
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Days Remaining: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", days_left),
                Style::default().fg(colors.cyan()).add_modifier(Modifier::BOLD),
            ),
        ],
    );
    row += 2;

    // Financial status
    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Cash: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("${}", state.cash),
                Style::default().fg(colors.green()).add_modifier(Modifier::BOLD),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Debt: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("${}", state.debt),
                Style::default().fg(colors.red()),
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Health: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}/100", state.health),
                if state.health > 50 {
                    Style::default().fg(colors.green())
                } else if state.health > 25 {
                    Style::default().fg(colors.yellow())
                } else {
                    Style::default().fg(colors.red())
                },
            ),
        ],
    );
    row += 1;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Guns: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("{}", state.guns),
                Style::default().fg(colors.cyan()),
            ),
        ],
    );
    row += 2;

    // Inventory
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "Inventory:",
            Style::default().fg(colors.grey()),
        )],
    );
    row += 1;

    if state.inventory.items.is_empty() {
        view.render_row(
            frame,
            row,
            vec![Span::styled(
                "  (empty)",
                Style::default().fg(colors.grey()),
            )],
        );
        row += 1;
    } else {
        for (product, quantity) in &state.inventory.items {
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(
                        format!("{:<12} ", product.name()),
                        Style::default().fg(colors.cyan()),
                    ),
                    Span::styled(
                        format!("{} units", quantity),
                        Style::default().fg(colors.fg()),
                    ),
                ],
            );
            row += 1;
        }
    }

    row += 1;

    // Net worth
    let inventory_value: i64 = state.inventory.items.iter()
        .map(|(product, quantity)| {
            let (min, max) = product.base_price_range();
            let avg_price = (min + max) / 2;
            avg_price * (*quantity as i64)
        })
        .sum();

    let net_worth = state.cash + inventory_value - state.debt;

    view.render_row(
        frame,
        row,
        vec![
            Span::styled("Est. Net Worth: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("${}", net_worth),
                if net_worth >= 0 {
                    Style::default().fg(colors.green()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors.red())
                },
            ),
        ],
    );

    // Help text
    view.render_help(frame, vec![("Esc/Enter", "back")]);
}

fn draw_event(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &DopeWarsState,
    colors: &ThemeColors,
) {
    use crate::plugins::games::dopewars::RandomEvent;

    let mut row = 5;

    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "═══════ EVENT ═════════════════════════════════════════════════════",
            Style::default().fg(colors.blue()),
        )],
    );
    row += 2;

    if let Some(ref msg) = state.message {
        view.render_row(
            frame,
            row,
            vec![Span::styled(msg, Style::default().fg(colors.yellow()))],
        );
        row += 2;
    }

    match &state.event {
        RandomEvent::CopsRaid { escaped, damage } => {
            row += 1;
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    if *escaped {
                        "You fought back with your guns!"
                    } else {
                        "The cops took some of your stash!"
                    },
                    Style::default().fg(colors.red()),
                )],
            );
            row += 2;
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("Damage taken: ", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{} HP", damage),
                        Style::default().fg(colors.red()),
                    ),
                    Span::styled("  Health: ", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{}/100", state.health),
                        if state.health > 50 {
                            Style::default().fg(colors.green())
                        } else {
                            Style::default().fg(colors.yellow())
                        },
                    ),
                ],
            );

            view.render_help(frame, vec![("Esc/Enter/Space", "continue")]);
        }
        RandomEvent::FindStash { product, quantity } => {
            row += 1;
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("Found: ", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{} {} ", quantity, product.name()),
                        Style::default().fg(colors.green()).add_modifier(Modifier::BOLD),
                    ),
                ],
            );

            view.render_help(frame, vec![("Esc/Enter/Space", "continue")]);
        }
        RandomEvent::Mugged { amount, damage } => {
            row += 1;
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("Lost: $", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{}", amount),
                        Style::default().fg(colors.red()),
                    ),
                    Span::styled("  Damage: ", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{} HP", damage),
                        Style::default().fg(colors.red()),
                    ),
                ],
            );
            row += 1;
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("Health: ", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{}/100", state.health),
                        if state.health > 50 {
                            Style::default().fg(colors.green())
                        } else if state.health > 25 {
                            Style::default().fg(colors.yellow())
                        } else {
                            Style::default().fg(colors.red())
                        },
                    ),
                ],
            );

            view.render_help(frame, vec![("Esc/Enter/Space", "continue")]);
        }
        RandomEvent::LoanShark { paid_off } => {
            row += 1;
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("Paid: $", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{}", paid_off),
                        Style::default().fg(colors.red()),
                    ),
                    Span::styled("  Debt reduced by: $", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{}", paid_off * 2),
                        Style::default().fg(colors.green()).add_modifier(Modifier::BOLD),
                    ),
                ],
            );
            row += 1;
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("New debt: $", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{}", state.debt),
                        Style::default().fg(colors.red()),
                    ),
                ],
            );

            view.render_help(frame, vec![("Esc/Enter/Space", "continue")]);
        }
        RandomEvent::GunShop => {
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    "Guns are $400 each. You can carry up to 10.",
                    Style::default().fg(colors.fg()),
                )],
            );
            row += 1;
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("Your cash: $", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{}", state.cash),
                        Style::default().fg(colors.green()),
                    ),
                    Span::styled("  Current guns: ", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{}", state.guns),
                        Style::default().fg(colors.cyan()),
                    ),
                ],
            );

            view.render_help(
                frame,
                vec![("G", "buy guns"), ("N", "no thanks")],
            );
        }
        RandomEvent::OfficerOffer { bribe } => {
            row += 1;
            view.render_row(
                frame,
                row,
                vec![Span::styled(
                    "Pay up or face the consequences!",
                    Style::default().fg(colors.red()),
                )],
            );
            row += 2;
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("Your cash: $", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{}", state.cash),
                        Style::default().fg(colors.green()),
                    ),
                    Span::styled("  Bribe: $", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{}", bribe),
                        Style::default().fg(colors.red()),
                    ),
                ],
            );
            row += 1;
            view.render_row(
                frame,
                row,
                vec![
                    Span::styled("Your guns: ", Style::default().fg(colors.grey())),
                    Span::styled(
                        format!("{}", state.guns),
                        Style::default().fg(colors.cyan()),
                    ),
                    Span::styled(
                        if state.guns >= 3 {
                            " (you could fight!)"
                        } else {
                            " (not enough to fight)"
                        },
                        Style::default().fg(colors.grey()),
                    ),
                ],
            );

            view.render_help(
                frame,
                vec![("P", "pay bribe"), ("F", "fight/refuse")],
            );
        }
        _ => {
            view.render_help(frame, vec![("Esc/Enter/Space", "continue")]);
        }
    }
}
