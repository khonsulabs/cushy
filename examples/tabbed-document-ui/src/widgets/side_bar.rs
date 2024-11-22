use cushy::figures::units::{Lp, Px};
use cushy::styles::{Color, Edges};
use cushy::styles::components::WidgetBackground;
use cushy::value::{Dynamic, Switchable};
use cushy::widget::{MakeWidget, MakeWidgetList, WidgetInstance};
use cushy::widgets::{Grid, Space};
use cushy::widgets::grid::{GridDimension, GridWidgets};
use cushy::widgets::label::{Displayable, LabelOverflow};

static EXTREMELY_DARK_GREY: Color = Color::new(0x24, 0x24, 0x24, 255);

#[derive(Clone)]
pub struct SideBarItem {
    label: String,
    value: Dynamic<Option<String>>,
}

#[derive(Default)]
pub struct SideBar {
    items: Vec<SideBarItem>,
    grid_dimensions: Dynamic<[GridDimension;2]>
}

impl SideBar {

    pub fn with_fixed_width_columns(self) -> Self {
        Self {
            items: self.items,
            grid_dimensions: Dynamic::new([
                // label
                GridDimension::Measured { size: Lp::new(100).into() },
                // value
                GridDimension::Measured { size: Lp::new(150).into() }
            ]),
        }
    }

    pub fn push(&mut self, item: SideBarItem) {
        self.items.push(item);
    }

    pub fn make_widget(&self) -> WidgetInstance {

        let grid_rows: Vec<(WidgetInstance, WidgetInstance)> = self.items.iter().map(|item|{
            (
                item.label.clone()
                    .into_label()
                    .overflow(LabelOverflow::Clip)
                    .make_widget(),
                item.value.clone().switcher(
                    move |value,_|{
                        match value {
                            Some(value) =>
                                value.clone()
                                    .into_label()
                                    .overflow(LabelOverflow::Clip)
                                    .make_widget()
                            ,
                            None =>
                                Space::clear()
                                    .make_widget(),
                        }
                    }
                )
                    .align_left()
                    .make_widget()
            )
        }).collect();

        let grid_row_widgets = GridWidgets::from(grid_rows);

        let grid = Grid::from_rows(grid_row_widgets);

        let grid_widget = grid
            .dimensions(self.grid_dimensions.clone())
            .align_top()
            .make_widget();

        let scrollable_content = grid_widget
            .with(&WidgetBackground, Color::GREEN) // grid background
            .vertical_scroll()
            .with(&WidgetBackground, Color::YELLOW) // colors the scrollbar itself
            .expand_vertically()
            .make_widget();

        let sidebar_header = "Sidebar Header"
            .into_label()
            .centered()
            .align_left()
            .with(&WidgetBackground, Color::MAROON) // label color
            .pad_by(Edges::default().with_bottom(Px::new(1)))
            .background_color(Color::LIMEGREEN); // padding color

        let sidebar_footer = "Sidebar Footer"
            .into_label()
            .centered()
            .align_left()
            .with(&WidgetBackground, Color::MAROON) // label color
            .pad_by(Edges::default().with_top(Px::new(1)))
            .background_color(Color::LIMEGREEN); // padding color

        let sidebar_widget = sidebar_header
            .and(scrollable_content)
            .and(sidebar_footer)
            .into_rows()
            .gutter(Px::new(0))
            .with(&WidgetBackground, Color::ORANGE) // colors header/footer and empty space below the grid
            // required so that when the background of the sidebar fills the container
            .expand_vertically()
            .with(&WidgetBackground, Color::PURPLE) // no effect
            //.with(&WidgetBackground, EXTREMELY_DARK_GREY)
            .make_widget();

        sidebar_widget
    }
}

impl SideBarItem {
    pub fn new(label: String, value: Dynamic<Option<String>>) -> Self {
        Self {
            label,
            value,
        }
    }
}

