use cushy::figures::units::{Lp};
use cushy::styles::{Color, DimensionRange};
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
            // FIXME how to color the space below the grid?
            .and(Space::colored(Color::RED)
                .make_widget()
            )
            .into_rows()
            .vertical_scroll()
            .expand_vertically()
            .make_widget();


        let sidebar_widget = "Sidebar Header".into_label()
            .and(scrollable_content)
            .and("Sidebar Footer")
            .into_rows()
            // required so that when the background of the sidebar fills the container
            .expand_vertically()
            .with(&WidgetBackground, EXTREMELY_DARK_GREY)
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

