use gooey_core::style::StyleComponent;
use gooey_core::{Children, Widget, WidgetValue};

#[derive(Debug, Widget)]
#[widget(authority = gooey)]
pub struct Flex {
    pub direction: WidgetValue<FlexDirection>,
    pub children: WidgetValue<Children>,
}

impl Flex {
    pub fn new(
        direction: impl Into<WidgetValue<FlexDirection>>,
        children: impl Into<WidgetValue<Children>>,
    ) -> Self {
        Self {
            direction: direction.into(),
            children: children.into(),
        }
    }

    pub fn columns(children: impl Into<WidgetValue<Children>>) -> Self {
        Self::new(FlexDirection::columns(), children)
    }

    pub fn rows(children: impl Into<WidgetValue<Children>>) -> Self {
        Self::new(FlexDirection::rows(), children)
    }
}

#[derive(Debug, StyleComponent)]
#[style(authority = gooey)]
pub enum FlexDirection {
    Row { reverse: bool },
    Column { reverse: bool },
}

impl FlexDirection {
    pub const fn columns() -> Self {
        Self::Column { reverse: false }
    }

    pub const fn columns_rev() -> Self {
        Self::Column { reverse: true }
    }

    pub const fn rows() -> Self {
        Self::Row { reverse: false }
    }

    pub const fn rows_rev() -> Self {
        Self::Row { reverse: true }
    }
}

#[derive(Default, Debug)]
pub struct FlexConfig {
    pub basis: u32,
    pub align: Option<SelfAlign>,
    pub justify: Option<SelfJustify>,
}

#[derive(Debug)]
pub enum SelfAlign {
    Stretch,
    Start,
    End,
    Center,
    Baseline,
    FirstBaseline,
    LastBaseline,
}

#[derive(Debug)]
pub enum SelfJustify {}

#[derive(Default, Debug)]
pub struct FlexTransmogrifier;

#[cfg(feature = "web")]
mod web {
    use gooey_core::reactor::Value;
    use gooey_core::{WidgetTransmogrifier, WidgetValue};
    use gooey_web::{WebApp, WebContext};
    use stylecs::Style;
    use wasm_bindgen::JsCast;
    use web_sys::{HtmlElement, Node};

    use crate::flex::FlexTransmogrifier;
    use crate::Flex;

    impl WidgetTransmogrifier<WebApp> for FlexTransmogrifier {
        type Widget = Flex;

        fn transmogrify(
            &self,
            widget: &Self::Widget,
            style: Value<Style>,
            context: &WebContext,
        ) -> Node {
            log::info!("instantiating flex");
            let mut tracked_children = Vec::new();
            let document = web_sys::window()
                .expect("no window")
                .document()
                .expect("no document");
            let container = document
                .create_element("div")
                .expect("failed to create button")
                .dyn_into::<HtmlElement>()
                .expect("incorrect element type");
            widget.children.map_ref(|children| {
                for (id, child) in children.entries() {
                    let child = context.instantiate(child);
                    container
                        .append_child(&child)
                        .expect("error appending child");
                    tracked_children.push((id, child));
                }
            });

            if let WidgetValue::Value(children) = widget.children {
                let container = container.clone();
                let context = context.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut children = children.into_stream();
                    while children.wait_next().await {
                        children.map_ref(|children| {
                            'children: for (index, (id, child)) in children.entries().enumerate() {
                                for tracked_index in index..tracked_children.len() {
                                    if tracked_children[tracked_index].0 == id {
                                        // This node already exists, move it in
                                        // the array if needed.
                                        if index != tracked_index {
                                            tracked_children.swap(tracked_index, index);
                                        }
                                        continue 'children;
                                    }
                                }

                                // The child wasn't found.
                                let child = context.instantiate(child);
                                if let Some(next_node) = tracked_children.get(index + 1) {
                                    container.insert_before(&child, Some(&next_node.1)).unwrap();
                                } else {
                                    container.append_child(&child).unwrap();
                                }
                                tracked_children.insert(index, (id, child));
                            }

                            for (_, removed) in tracked_children.drain(children.len()..) {
                                container.remove_child(&removed).unwrap();
                            }
                        });
                    }
                });
            }
            container.into()
        }
    }
}

#[cfg(feature = "raster")]
mod raster {
    use std::collections::HashSet;
    use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};

    use alot::LotId;
    use gooey_core::graphics::Point;
    use gooey_core::math::{FloatConversion, Rect, Size};
    use gooey_core::style::{Px, UPx};
    use gooey_core::{WidgetTransmogrifier, WidgetValue};
    use gooey_raster::{
        RasterContext, Rasterizable, RasterizedApp, Renderer, SurfaceHandle, WidgetRasterizer,
    };
    use taffy::node::MeasureFunc;
    use taffy::style::AvailableSpace;
    use taffy::Taffy;

    use crate::flex::FlexTransmogrifier;
    use crate::Flex;

    struct FlexRasterizer {
        children: RasterizedChildren,
        flex: FlexLayout,
        layouts: Vec<taffy::layout::Layout>,
        mouse_tracking: Option<LotId>,
        hovering: HashSet<LotId>,
    }

    #[derive(Default)]
    struct RasterizedChildren(Arc<Mutex<Vec<(LotId, Rasterizable)>>>);

    impl<Surface> WidgetTransmogrifier<RasterizedApp<Surface>> for FlexTransmogrifier
    where
        Surface: gooey_raster::Surface,
    {
        type Widget = Flex;

        fn transmogrify(
            &self,
            widget: &Self::Widget,
            style: gooey_core::reactor::Value<stylecs::Style>,
            context: &RasterContext<Surface>,
        ) -> Rasterizable {
            let raster_children = RasterizedChildren::default();
            let mut locked_children = raster_children
                .0
                .lock()
                .map_or_else(PoisonError::into_inner, |g| g);
            let flex = FlexLayout::new(taffy::style::Style::DEFAULT);
            widget.children.map_ref(|children| {
                for (id, child) in children.entries() {
                    flex.push_child(id, taffy::style::Style::DEFAULT);
                    locked_children.push((
                        id,
                        context
                            .widgets()
                            .instantiate(child.widget.as_ref(), style, context),
                    ));
                }
            });
            drop(locked_children);

            if let WidgetValue::Value(value) = &widget.children {
                value.for_each({
                    let handle = context.handle().clone();
                    move |_| {
                        handle.invalidate();
                    }
                })
            }

            Rasterizable::new(FlexRasterizer {
                children: raster_children,
                flex,
                layouts: Vec::new(),
                mouse_tracking: None,
                hovering: HashSet::new(),
            })
        }
    }

    impl WidgetRasterizer for FlexRasterizer {
        type Widget = Flex;

        fn measure(
            &mut self,
            available_space: Size<Option<UPx>>,
            renderer: &mut dyn Renderer,
        ) -> Size<UPx> {
            todo!()
        }

        fn draw(&mut self, renderer: &mut dyn Renderer) {
            let mut children = self
                .children
                .0
                .lock()
                .map_or_else(PoisonError::into_inner, |g| g);
            self.layouts = self
                .flex
                .measurer
                .measure(&self.flex, renderer.size(), |request| {
                    let child = children
                        .iter_mut()
                        .find_map(|(id, child)| (*id == request.id).then_some(child))
                        .expect("unknown child");
                    child.measure(
                        Size::new(
                            request.size.width.map(UPx::from),
                            request.size.height.map(UPx::from),
                        ),
                        renderer,
                    )
                });

            for (layout, (_id, rasterizable)) in self.layouts.iter().zip(children.iter_mut()) {
                renderer.clip_to(Rect::new(
                    Point::new(layout.location.x.into(), layout.location.y.into()),
                    Size::new(layout.size.width, layout.size.height),
                ));
                rasterizable.draw(renderer);
                renderer.pop_clip();
            }
        }

        fn mouse_down(&mut self, location: Point<Px>, surface: &dyn SurfaceHandle) {
            let mut children = self
                .children
                .0
                .lock()
                .map_or_else(PoisonError::into_inner, |g| g);
            for (layout, (id, rasterizable)) in self.layouts.iter().zip(children.iter_mut()) {
                let rect = Rect::new(
                    Point::new(Px::from(layout.location.x), Px::from(layout.location.y)),
                    Size::new(Px::from(layout.size.width), Px::from(layout.size.height)),
                );
                let relative = location - rect.origin;
                if relative.x >= 0
                    && relative.y >= 0
                    && relative.x < rect.size.width
                    && relative.y < rect.size.height
                {
                    self.mouse_tracking = Some(*id);
                    rasterizable.mouse_down(relative, surface);
                    break;
                }
            }
        }

        fn cursor_moved(&mut self, location: Option<Point<Px>>, surface: &dyn SurfaceHandle) {
            let mut children = self
                .children
                .0
                .lock()
                .map_or_else(PoisonError::into_inner, |g| g);
            for (layout, (id, rasterizable)) in self.layouts.iter().zip(children.iter_mut()) {
                let rect = Rect::new(
                    Point::new(Px::from(layout.location.x), Px::from(layout.location.y)),
                    Size::new(Px::from(layout.size.width), Px::from(layout.size.height)),
                );
                let relative = location.map(|location| location - rect.origin);
                if relative.map_or(false, |relative| {
                    relative.x >= 0
                        && relative.y >= 0
                        && relative.x < rect.size.width
                        && relative.y < rect.size.height
                }) {
                    rasterizable.cursor_moved(relative, surface);
                    self.hovering.insert(*id);
                } else if self.hovering.remove(id) {
                    rasterizable.cursor_moved(None, surface);
                }
            }
        }

        fn mouse_up(&mut self, location: Option<Point<Px>>, surface: &dyn SurfaceHandle) {
            let mut children = self
                .children
                .0
                .lock()
                .map_or_else(PoisonError::into_inner, |g| g);
            if let Some((layout, (_, rasterizable))) = self
                .layouts
                .iter()
                .zip(children.iter_mut())
                .find(|(_, (id, _))| Some(*id) == self.mouse_tracking)
            {
                let rect = Rect::new(
                    Point::new(Px::from(layout.location.x), Px::from(layout.location.y)),
                    Size::new(Px::from(layout.size.width), Px::from(layout.size.height)),
                );
                let relative = location.map(|location| location - rect.origin);
                if relative.map_or(false, |relative| {
                    relative.x >= 0
                        && relative.y >= 0
                        && relative.x < rect.size.width
                        && relative.y < rect.size.height
                }) {
                    rasterizable.mouse_up(relative, surface);
                } else {
                    rasterizable.mouse_up(None, surface);
                }
            }
            self.mouse_tracking = None;
        }
    }

    struct FlexLayout {
        results: (flume::Sender<LayoutResult>, flume::Receiver<LayoutResult>),
        nodes: Arc<Mutex<FlexNodes>>,
        measurer: Measurer,
    }

    impl FlexLayout {
        pub fn new(root_style: taffy::style::Style) -> Self {
            let results = flume::unbounded();
            TaffyThread::send(
                CommandKind::NewNode {
                    measure_id: None,
                    style: root_style,
                },
                results.0.clone(),
            );
            let Ok(Ok(LayoutOutput::Node(root))) = results.1.recv() else { unreachable!("unexpected response from thread") };

            Self {
                results,
                nodes: Arc::new(Mutex::new(FlexNodes {
                    root,
                    children: Vec::new(),
                    dirty: false,
                })),
                measurer: Measurer::new(),
            }
        }

        pub fn push_child(&self, id: LotId, style: taffy::style::Style) {
            TaffyThread::send(
                CommandKind::NewNode {
                    style,
                    measure_id: Some(id),
                },
                self.results.0.clone(),
            );

            let Ok(Ok(LayoutOutput::Node(node))) = self.results.1.recv() else { unreachable!("unexpected response from thread") };
            let mut nodes = self
                .nodes
                .lock()
                .map_or_else(PoisonError::into_inner, |g| g);
            nodes.children.push(node);
            nodes.dirty = true;
        }
    }

    struct FlexNodes {
        root: taffy::node::Node,
        children: Vec<taffy::node::Node>,
        dirty: bool,
    }

    impl Drop for FlexNodes {
        fn drop(&mut self) {
            for child in self.children.drain(..) {
                TaffyThread::send(CommandKind::Remove(child), None);
            }
            TaffyThread::send(CommandKind::Remove(self.root), None);
        }
    }

    struct TaffyThread {
        sender: flume::Sender<LayoutCommand>,
    }

    impl TaffyThread {
        pub fn new() -> Self {
            let (sender, receiver) = flume::unbounded();
            std::thread::Builder::new()
                .name(String::from("layout"))
                .spawn(move || Self::run(receiver))
                .expect("error spawning layout thread");
            TaffyThread { sender }
        }

        fn run(messages: flume::Receiver<LayoutCommand>) {
            let mut taffy = Taffy::new();
            while let Ok(message) = messages.recv() {
                let result = match message.kind {
                    CommandKind::Layout {
                        nodes,
                        space,
                        measurer: _measurer,
                    } => Self::compute_layout(&mut taffy, &nodes, space),
                    CommandKind::NewNode { style, measure_id } => {
                        Self::new_node(&mut taffy, measure_id, style)
                    }
                    CommandKind::Remove(_) => todo!(),
                    CommandKind::InsertChild { child, parent } => todo!(),
                    CommandKind::RemoveChildAtIndex { index, parent } => todo!(),
                };
                if let Some(results) = message.results {
                    let _result = results.send(result);
                }
            }
        }

        fn compute_layout(
            taffy: &mut Taffy,
            nodes: &Arc<Mutex<FlexNodes>>,
            space: Size<UPx>,
        ) -> LayoutResult {
            let mut nodes = nodes.lock().map_or_else(PoisonError::into_inner, |g| g);
            if nodes.dirty {
                taffy.set_children(nodes.root, &nodes.children)?;
                nodes.dirty = false;
            }
            taffy.compute_layout(
                nodes.root,
                taffy::geometry::Size {
                    width: AvailableSpace::Definite(space.width.into_float()),
                    height: AvailableSpace::Definite(space.height.into_float()),
                },
            )?;
            let mut layouts = Vec::with_capacity(nodes.children.len());
            for &child in &nodes.children {
                layouts.push(*taffy.layout(child)?);
            }
            Ok(LayoutOutput::Layouts(layouts))
        }

        fn new_node(
            taffy: &mut Taffy,
            measure_id: Option<LotId>,
            style: taffy::style::Style,
        ) -> LayoutResult {
            let node = if let Some(id) = measure_id {
                taffy.new_leaf_with_measure(
                    style,
                    MeasureFunc::Boxed(Box::new(move |a, b| GlobalMeasurer::measure(id, a, b))),
                )?
            } else {
                taffy.new_leaf(style)?
            };
            Ok(LayoutOutput::Node(node))
        }
    }

    impl TaffyThread {
        fn global() -> &'static TaffyThread {
            static TAFFY: OnceLock<TaffyThread> = OnceLock::new();
            TAFFY.get_or_init(TaffyThread::new)
        }

        pub fn send(kind: CommandKind, results: impl Into<Option<flume::Sender<LayoutResult>>>) {
            Self::global()
                .sender
                .send(LayoutCommand {
                    kind,
                    results: results.into(),
                })
                .expect("layout thread isn't running")
        }
    }

    struct LayoutCommand {
        kind: CommandKind,
        results: Option<flume::Sender<LayoutResult>>,
    }

    enum CommandKind {
        Layout {
            nodes: Arc<Mutex<FlexNodes>>,
            space: Size<UPx>,
            measurer: MeasureGuard,
        },
        NewNode {
            measure_id: Option<LotId>,
            style: taffy::style::Style,
        },
        Remove(taffy::node::Node),
        InsertChild {
            child: taffy::node::Node,
            parent: taffy::node::Node,
        },
        RemoveChildAtIndex {
            index: usize,
            parent: taffy::node::Node,
        },
    }

    enum LayoutOutput {
        Node(taffy::node::Node),
        Layouts(Vec<taffy::layout::Layout>),
    }

    type LayoutResult = Result<LayoutOutput, taffy::error::TaffyError>;

    pub struct MeasureGuard;

    impl Drop for MeasureGuard {
        fn drop(&mut self) {
            GlobalMeasurer::uninstall();
        }
    }

    struct Measurer {
        requests: (
            flume::Sender<MeasureRequest>,
            flume::Receiver<MeasureRequest>,
        ),
        sizes: (flume::Sender<Size<UPx>>, flume::Receiver<Size<UPx>>),
    }

    impl Measurer {
        pub fn new() -> Self {
            let requests = flume::bounded(1);
            let sizes = flume::bounded(1);
            Self { requests, sizes }
        }

        pub fn measure(
            &self,
            layout: &FlexLayout,
            space: Size<UPx>,
            mut measure: impl FnMut(MeasureRequest) -> Size<UPx>,
        ) -> Vec<taffy::layout::Layout> {
            let (requests_sender, requests_receiver) = flume::bounded(1);
            let measurer = GlobalMeasurer::install(requests_sender, self.sizes.1.clone());
            TaffyThread::send(
                CommandKind::Layout {
                    nodes: layout.nodes.clone(),
                    space,
                    measurer,
                },
                layout.results.0.clone(),
            );
            while let Ok(request) = requests_receiver.recv() {
                self.sizes
                    .0
                    .send(measure(request))
                    .expect("thread should be waiting");
            }
            let Ok(Ok(LayoutOutput::Layouts(layouts))) = layout.results.1.recv() else { unreachable!("error on taffy thread") };
            layouts
        }
    }

    struct GlobalMeasurer {
        requests: flume::Sender<MeasureRequest>,
        sizes: flume::Receiver<Size<UPx>>,
    }

    #[derive(Default)]
    struct GlobalMeasureState {
        measurer: Mutex<Option<GlobalMeasurer>>,
        sync: Condvar,
    }

    static GLOBAL_MEASURE: OnceLock<GlobalMeasureState> = OnceLock::new();
    impl GlobalMeasurer {
        pub fn install(
            request_sender: flume::Sender<MeasureRequest>,
            size_receiver: flume::Receiver<Size<UPx>>,
        ) -> MeasureGuard {
            let global = GLOBAL_MEASURE.get_or_init(GlobalMeasureState::default);
            let mut measurer = global
                .measurer
                .lock()
                .map_or_else(PoisonError::into_inner, |g| g);
            while measurer.is_some() {
                measurer = global
                    .sync
                    .wait(measurer)
                    .map_or_else(PoisonError::into_inner, |g| g)
            }
            *measurer = Some(GlobalMeasurer {
                requests: request_sender,
                sizes: size_receiver,
            });
            MeasureGuard
        }

        fn uninstall() {
            let global = GLOBAL_MEASURE.get().expect("guard requires initialization");
            let mut measurer = global
                .measurer
                .lock()
                .map_or_else(PoisonError::into_inner, |g| g);
            *measurer = None;
            global.sync.notify_one();
        }

        pub fn measure(
            id: LotId,
            size: taffy::geometry::Size<Option<f32>>,
            available: taffy::geometry::Size<AvailableSpace>,
        ) -> taffy::geometry::Size<f32> {
            let measurer = GLOBAL_MEASURE
                .get()
                .expect("guard requires initialization")
                .measurer
                .lock()
                .map_or_else(PoisonError::into_inner, |g| g);
            let measurer = measurer.as_ref().expect("no measure channels");
            measurer
                .requests
                .send(MeasureRequest {
                    id,
                    size,
                    available,
                })
                .expect("global measurer disconnected");
            let size = measurer
                .sizes
                .recv()
                .expect("global measurer sizes disconnected");
            taffy::geometry::Size {
                width: size.width.into_float(),
                height: size.height.into_float(),
            }
        }
    }

    #[derive(Debug)]
    struct MeasureRequest {
        id: LotId,
        size: taffy::geometry::Size<Option<f32>>,
        available: taffy::geometry::Size<AvailableSpace>,
    }
}
