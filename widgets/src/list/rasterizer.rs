use std::collections::HashMap;

use gooey_core::{
    figures::{Figure, Point, Rectlike, Size, SizedRect, Vector},
    styles::{Style, TextColor},
    Scaled, Transmogrifier, TransmogrifierContext, WidgetRegistration,
};
use gooey_rasterizer::{ContentArea, Rasterizer, Renderer, WidgetRasterizer};
use gooey_text::{prepared::PreparedText, wrap::TextWrap, Text};

use super::{ItemLabelIterator, Kind, ListAdornmentSpacing};
use crate::list::{List, ListTransmogrifier};

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for ListTransmogrifier {
    type State = State;
    type Widget = List;

    fn receive_command(
        &self,
        _command: <Self::Widget as gooey_core::Widget>::Command,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
    ) {
        context.frontend.set_needs_redraw();
    }
}

impl<R: Renderer> WidgetRasterizer<R> for ListTransmogrifier {
    fn render(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        content_area: &ContentArea,
    ) {
        let renderer = context.frontend.renderer().unwrap();
        let bounds = content_area.bounds();
        let indicators = context
            .state
            .indicators(
                &context.widget.kind,
                context.widget.children.len(),
                renderer,
                context.style.as_ref(),
            )
            .collect::<Vec<_>>();
        let max_indicator_width = indicators
            .iter()
            .filter_map(|text| text.as_ref().map(|t| t.size().width()))
            .reduce(Figure::max);
        let spacing = context.style.get_or_default::<ListAdornmentSpacing>().0;
        let offset_amount = max_indicator_width
            .map_or_else(Figure::default, |max_indicator_width| {
                max_indicator_width + spacing
            });

        let mut indicators = indicators.into_iter();
        for_each_measured_widget(
            context,
            bounds.size() - Vector::from_x(offset_amount),
            |child, mut child_bounds| {
                child_bounds =
                    child_bounds.translate(content_area.location + Vector::from_x(offset_amount));

                if let Some(indicator) = indicators.next().flatten() {
                    indicator.render::<TextColor, _>(
                        renderer,
                        child_bounds.origin - Vector::from_x(spacing + indicator.size().width()),
                        true,
                        Some(context.style()),
                    );
                }

                context.frontend.with_transmogrifier(
                    child.id(),
                    |transmogrifier, mut child_context| {
                        transmogrifier.render_within(
                            &mut child_context,
                            child_bounds.as_rect(),
                            Some(context.registration.id()),
                            context.style(),
                        );
                    },
                );
            },
        );
    }

    fn measure_content(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size<Option<f32>, Scaled>,
    ) -> Size<f32, Scaled> {
        let mut size = Size::<f32, Scaled>::default();
        let renderer = context.frontend.renderer().unwrap();
        let context_size = renderer.size();
        let spacing = context.style.get_or_default::<ListAdornmentSpacing>().0;
        context.state.clear_indicator_state();
        let max_indicator_width = context
            .state
            .indicators(
                &context.widget.kind,
                context.widget.children.len(),
                renderer,
                context.style.as_ref(),
            )
            .filter_map(|text| text.as_ref().map(|t| t.size().width()))
            .reduce(Figure::max);
        let offset_amount = max_indicator_width
            .map(|width| spacing + width)
            .unwrap_or_default();

        let constrained_size = Size::new(
            constraints.width.unwrap_or(context_size.width) - offset_amount.get(),
            constraints.height.unwrap_or(context_size.height),
        );
        for_each_measured_widget(context, constrained_size, |_layout, child_bounds| {
            size.width = size.width.max(child_bounds.size.width);
            size.height += child_bounds.size.height;
        });
        size.width += offset_amount.get();
        size
    }
}

#[allow(clippy::cast_precision_loss)]
fn for_each_measured_widget<R: Renderer, F: FnMut(&WidgetRegistration, SizedRect<f32, Scaled>)>(
    context: &TransmogrifierContext<'_, ListTransmogrifier, Rasterizer<R>>,
    constraints: Size<f32, Scaled>,
    callback: F,
) {
    for_each_widget(
        &context.widget.children,
        |child| {
            context
                .frontend
                .with_transmogrifier(child.id(), |transmogrifier, mut child_context| {
                    let child_size = transmogrifier
                        .content_size(
                            &mut child_context,
                            Size::from_width(Some(constraints.width)),
                        )
                        .total_size();
                    Size::new(constraints.width, child_size.height)
                })
                .unwrap_or_default()
        },
        callback,
    );
}

#[allow(clippy::cast_precision_loss)]
fn for_each_widget<
    F: FnMut(&WidgetRegistration, SizedRect<f32, Scaled>),
    W: Fn(&WidgetRegistration) -> Size<f32, Scaled>,
>(
    children: &[WidgetRegistration],
    child_measurer: W,
    mut callback: F,
) {
    let mut top = Figure::default();
    for child in children {
        let child_size = child_measurer(child).max(&Size::default());
        callback(child, SizedRect::new(Point::from_y(top), child_size));
        top += child_size.height();
    }
}

#[derive(Debug, Default)]
pub struct State {
    indicators: HashMap<Option<i32>, PreparedText>,
}

impl State {
    fn clear_indicator_state(&mut self) {
        self.indicators.clear();
    }

    #[allow(clippy::map_entry)]
    fn indicator(
        &mut self,
        value: Option<i32>,
        label: &str,
        renderer: &impl Renderer,
        context_style: &Style,
    ) -> Option<PreparedText> {
        if !self.indicators.contains_key(&value) {
            let text = Text::from(label);
            self.indicators.insert(
                value,
                text.wrap(renderer, TextWrap::NoWrap, Some(context_style)),
            );
        }

        self.indicators.get(&value).cloned()
    }

    fn indicators<'a, R: Renderer>(
        &'a mut self,
        kind: &'a Kind,
        count: usize,
        renderer: &'a R,
        context_style: &'a Style,
    ) -> PreparedLabelIterator<'a, R> {
        PreparedLabelIterator {
            state: self,
            labels: ItemLabelIterator::new(kind, count),
            renderer,
            context_style,
        }
    }
}

struct PreparedLabelIterator<'a, R: Renderer> {
    labels: ItemLabelIterator<'a>,
    state: &'a mut State,
    renderer: &'a R,
    context_style: &'a Style,
}

impl<'a, R: Renderer> Iterator for PreparedLabelIterator<'a, R> {
    type Item = Option<PreparedText>;

    fn next(&mut self) -> Option<Self::Item> {
        self.labels.next().map(|opt_label| {
            opt_label.and_then(|label| {
                self.state.indicator(
                    self.labels.value,
                    label.as_ref(),
                    self.renderer,
                    self.context_style,
                )
            })
        })
    }
}
