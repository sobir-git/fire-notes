use fire_ui_elegant_model::*;
use std::rc::Rc;

// Deterministic wrapping stand-in, not a claim about native shaping.
fn paragraph(limits: Limits) -> Metrics {
    let columns = (limits.width / 10.0).floor().max(1.0);
    let lines = (40.0 / columns).ceil();
    Metrics {
        size: Size::new(400.0_f32.min(limits.width), lines * 20.0),
        baseline: Some(15.0),
    }
}
#[test]
fn dialog_measures_its_content_and_decoration() {
    let content = |limits: Limits| {
        column(
            &[ChildLayout {
                content: &paragraph,
                width: Width::Fill,
            }],
            limits.width,
            8.0,
        )
        .metrics
    };
    let wide = padded(
        &content,
        Limits {
            width: 224.0,
            height: None,
        },
        12.0,
    );
    let narrow = padded(
        &content,
        Limits {
            width: 124.0,
            height: None,
        },
        12.0,
    );
    assert_eq!(wide.size, Size::new(224.0, 64.0));
    assert_eq!(narrow.size, Size::new(124.0, 104.0));
    assert_eq!(wide.baseline, Some(27.0));
}
#[test]
fn scroller_derives_extent_after_width_changes() {
    let wide = vertical_scroll(&paragraph, Size::new(200.0, 30.0));
    let narrow = vertical_scroll(&paragraph, Size::new(100.0, 30.0));
    assert_eq!(wide.max_offset, 10.0);
    assert_eq!(narrow.max_offset, 50.0);
    assert_eq!(narrow.content.size, Size::new(100.0, 80.0));
}
#[test]
fn arbitrary_content_and_text_share_a_baseline() {
    let icon = Metrics {
        size: Size::new(16.0, 16.0),
        baseline: None,
    };
    let text = paragraph(Limits {
        width: 400.0,
        height: None,
    });
    let row = baseline_row(&[icon, text], 6.0);
    assert_eq!(row.metrics.size, Size::new(422.0, 21.0));
    assert_eq!(row.children[0].y, 0.0);
    assert_eq!(row.children[1].y, 1.0);
    // A custom canvas has no special enum case or framework registration.
    let canvas = |_: Limits| Metrics {
        size: Size::new(30.0, 30.0),
        baseline: None,
    };
    let layout = column(
        &[
            ChildLayout {
                content: &canvas,
                width: Width::Natural,
            },
            ChildLayout {
                content: &canvas,
                width: Width::Fill,
            },
        ],
        100.0,
        4.0,
    );
    assert_eq!(layout.children[0].metrics.size.width, 30.0);
    assert_eq!(layout.children[1].metrics.size.width, 100.0);
}
#[test]
fn shared_appearance_preserves_local_overrides_and_classifies_invalidation() {
    let theme = Rc::new(Theme {
        text_size: 14.0,
        control_inset: 8.0,
        foreground: Color(20, 20, 20),
        accent: Color(200, 40, 10),
    });
    let base = Appearance {
        theme: Rc::clone(&theme),
        local: Overrides::default(),
    };
    let mut local = Appearance {
        theme,
        local: Overrides {
            foreground: Some(Color(10, 50, 80)),
            ..Overrides::default()
        },
    };
    assert!(Rc::ptr_eq(&base.theme, &local.theme));
    assert!(!local.resolve().requires_layout(base.resolve()));
    let before = local.resolve();
    local.theme = Rc::new(Theme {
        text_size: 18.0,
        ..(*local.theme).clone()
    });
    assert!(local.resolve().requires_layout(before));
    assert_eq!(local.resolve().foreground, Color(10, 50, 80));
    assert_eq!(local.resolve().text_size, 18.0);
}
