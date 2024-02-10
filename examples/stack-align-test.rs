use cushy::widget::MakeWidget;
use cushy::Run;

/// This example shows a tricky layout problem. The hierarchy of widgets is
/// this:
///
/// ```text
/// Expand (.expand())
/// | Align (.centered())
/// | | Stack (.into_rows())
/// | | | Label
/// | | | Align (.centered())
/// | | | | Button
/// ```
///
/// When the Stack widget attempted to implmement a single-pass layout, this
/// caused the Button to be aligned to the left inside of the stack. The Stack
/// widget now utilizes two `layout()` operations for layouts like this. Here's
/// the reasoning:
///
/// At the window root, we have an Align wrapped by an Expand. The Align widget
/// during layout asks its children to size-to-fit. This means the Stack is
/// asking its children to size-to-fit as well.
///
/// The Stack's orientation is Rows, and since the children are Resizes or
/// Expands, the widgets are size-to-fit. This means that the Stack will measure
/// these widgets asking them to size to fit.
///
/// After running this pass of measurement, we can assign the heights of each of
/// the rows to the measurements we received. The width of the stack becomes the
/// maximum width of all children measured.
///
/// In a single-pass layout, this means the Align widget inside of the Stack
/// never receives an opportunity to lay its children out with the final width.
/// The Button does end up centered because of this. Fixing it also becomes
/// tricky, because if surround the button in an Expand, it now instructs the
/// Stack to expand to fill its parent.
///
/// After some careful deliberation, @ecton reasoned that in the situation where
/// a Stack is asked to layout with the Stack's non-primary being a size-to-fit
/// measurement, a second layout call for all children is required with Known
/// measurements to allow layouts like this example to work correctly.
fn main() -> cushy::Result {
    // TODO once we have offscreen rendering, turn this into a test case
    "Really Long Label"
        .and("Short".into_button().centered())
        .into_rows()
        .contain()
        .centered()
        .expand()
        .run()
}
