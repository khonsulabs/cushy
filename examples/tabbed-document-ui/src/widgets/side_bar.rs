use cushy::figures::units::Px;
use cushy::styles::{Color, DimensionRange};
use cushy::styles::components::WidgetBackground;
use cushy::value::{Dynamic, Switchable};
use cushy::widget::{MakeWidget, WidgetInstance};
use cushy::widgets::{Grid, Space};
use cushy::widgets::grid::GridWidgets;
use cushy::widgets::label::Displayable;

static EXTREMELY_DARK_GREY: Color = Color::new(0x24, 0x24, 0x24, 255);

pub struct SideBarItem {
    label: String,
    value: Dynamic<Option<String>>,
}

#[derive(Default)]
pub struct SideBar {
    items: Vec<SideBarItem>
}

impl SideBar {
    pub fn push(&mut self, item: SideBarItem) {
        self.items.push(item);
    }
}

impl SideBar {
    pub fn make_widget(&self) -> WidgetInstance {

        let grid_rows: Vec<(WidgetInstance, WidgetInstance)> = self.items.iter().map(|item|{
            (
                item.label.clone().into_label().make_widget(),
                item.value.clone().switcher(
                    move |value,_|{
                        match value {
                            Some(value) => value.clone().into_label().make_widget(),
                            None => Space::clear().make_widget(),
                        }
                    }
                )
                    // FIXME ideally we want a sensible default width
                    .width(DimensionRange::from(Px::new(100)..Px::new(200)))
                    .make_widget()
            )
        }).collect();

        let grid_row_widgets = GridWidgets::from(grid_rows);

        let grid = Grid::from_rows(grid_row_widgets);

        grid
            .align_top()
            .with(&WidgetBackground, EXTREMELY_DARK_GREY)
            .make_widget()
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

