use std::sync::atomic::{AtomicU64, Ordering};

use hooke::{
    AlignContent, AlignItems, BoxSizing, Children, Display, Edges, FlexDirection, FlexWrap, Handle, JustifyContent,
    LayoutCx, LayoutNode as HookeNode, LayoutState, Length, Measure, MeasureInput, Measured, Opt, Parts, PartsMut,
    RootConstraints, Scratch, Style,
};

use crate::pangui_template::{BenchmarkSize, FlowDirection, LayoutNode, SizeMode, WrapMode};

static NEXT_VERSION: AtomicU64 = AtomicU64::new(1);

/// A fresh, non-zero props version.
fn fresh_version() -> u64 {
    NEXT_VERSION.fetch_add(1, Ordering::Relaxed)
}

/// A host node with versioned props: `style_key` is the version, so style changes are detected in O(1).
#[derive(Clone)]
struct Node {
    style: Style,
    version: u64,
    children: Children<Node>,
    layout: LayoutState,
}

impl HookeNode for Node {
    type Scalar = f32;
    fn parts(&self) -> Parts<'_, Self> {
        Parts { style: &self.style, state: &self.layout, children: &self.children }
    }
    fn parts_mut(&mut self) -> PartsMut<'_, Self> {
        PartsMut { style: &self.style, state: &mut self.layout, children: &mut self.children }
    }
    fn style_key(&self) -> u64 {
        self.version
    }
}

/// The templates have no measured leaves, so the measurer is never called.
struct NoMeasure;

impl Measure<Node> for NoMeasure {
    fn measure(&mut self, _: &mut Node, _: MeasureInput<f32>, _: &mut LayoutCx<'_, f32>) -> Measured<f32> {
        panic!("hooke called the measurer, but the PanGUI templates have no measured leaves")
    }
}

fn is_mode(size: &Option<BenchmarkSize>, mode: SizeMode) -> bool {
    size.is_some_and(|size| size.mode == mode)
}

fn dimension(size: &Option<BenchmarkSize>) -> Length<f32> {
    match size {
        Some(BenchmarkSize { mode: SizeMode::Pixels, value }) => Length::Px(*value),
        Some(BenchmarkSize { mode: SizeMode::Percentage, value }) => Length::Percent(*value),
        // `Expand` is translated to `flex-grow` or `align-items: stretch`, and `Ratio` to `aspect-ratio`
        Some(BenchmarkSize { mode: SizeMode::Fit | SizeMode::Expand | SizeMode::Ratio, .. }) | None => Length::Auto,
    }
}

/// A size bound; `unset` is the initial value (`auto` for minimums, `none` for maximums).
fn bound(size: &Option<BenchmarkSize>, property: &str, unset: Length<f32>) -> Length<f32> {
    match size {
        Some(BenchmarkSize { mode: SizeMode::Pixels, value }) => Length::Px(*value),
        Some(BenchmarkSize { mode: SizeMode::Percentage, value }) => Length::Percent(*value),
        Some(BenchmarkSize { mode, .. }) => panic!("{mode:?} is not supported as a value of {property}"),
        None => unset,
    }
}

/// Translate a node of a PanGUI template into a hooke style, one to one with the Taffy adapter's `style()`.
/// `parent_is_row` is whether the parent of the node lays out its children horizontally.
fn style(node: &LayoutNode, parent_is_row: bool) -> Style {
    let (main_size, main_min_size) =
        if parent_is_row { (&node.width, &node.min_width) } else { (&node.height, &node.min_height) };

    let expands = is_mode(main_size, SizeMode::Expand);
    let (flex_grow, flex_shrink, flex_basis) = match (main_size, main_min_size) {
        (Some(BenchmarkSize { mode: SizeMode::Expand, value }), _) => (*value, 1.0, Length::Px(0.0)),
        (_, Some(BenchmarkSize { mode: SizeMode::Expand, value })) => (*value, 0.0, Length::Auto),
        _ => (0.0, 0.0, Length::Auto),
    };

    let (mut min_width, mut min_height) = (Length::Auto, Length::Auto);
    if !(parent_is_row && is_mode(&node.min_width, SizeMode::Expand)) {
        min_width = bound(&node.min_width, "MinWidth", Length::Auto);
    }
    if !(!parent_is_row && is_mode(&node.min_height, SizeMode::Expand)) {
        min_height = bound(&node.min_height, "MinHeight", Length::Auto);
    }
    if expands {
        if parent_is_row && node.min_width.is_none() {
            min_width = Length::Px(0.0);
        }
        if !parent_is_row && node.min_height.is_none() {
            min_height = Length::Px(0.0);
        }
    }

    let aspect_ratio = match (&node.width, &node.height) {
        (Some(BenchmarkSize { mode: SizeMode::Ratio, value }), _) => Opt::some(*value),
        (_, Some(BenchmarkSize { mode: SizeMode::Ratio, value })) => Opt::some(1.0 / *value),
        _ => Opt::NONE,
    };

    let gap = Length::Px(node.gap.unwrap_or(0.0));
    let px = |v: Option<f32>| Length::Px(v.unwrap_or(0.0));
    Style::web()
        .with_box_sizing(BoxSizing::BorderBox)
        .with_display(Display::Flex)
        .with_flex_direction(match node.flow_direction {
            Some(FlowDirection::Horizontal) => FlexDirection::Row,
            Some(FlowDirection::Vertical) | None => FlexDirection::Column,
        })
        .with_flex_wrap(match node.wrap {
            Some(WrapMode::Auto) => FlexWrap::Wrap,
            Some(WrapMode::NoWrap) | None => FlexWrap::NoWrap,
        })
        .with_size(dimension(&node.width), dimension(&node.height))
        .with_min_size(min_width, min_height)
        .with_max_size(
            bound(&node.max_width, "MaxWidth", Length::None),
            bound(&node.max_height, "MaxHeight", Length::None),
        )
        .with_aspect_ratio(aspect_ratio)
        .with_gap(gap, gap)
        .with_padding(Edges {
            left: px(node.padding_left),
            right: px(node.padding_right),
            top: px(node.padding_top),
            bottom: px(node.padding_bottom),
        })
        .with_margin(Edges {
            left: px(node.margin_left),
            right: px(node.margin_right),
            top: px(node.margin_top),
            bottom: px(node.margin_bottom),
        })
        .with_flex(flex_grow, flex_shrink, flex_basis)
        .with_align_items(AlignItems::Stretch)
        .with_justify_content(JustifyContent::FlexStart)
        .with_align_content(AlignContent::FlexStart)
}

fn build_node(node: &LayoutNode, parent_is_row: bool) -> Handle<Node> {
    let horizontal = node.flow_direction == Some(FlowDirection::Horizontal);
    let children = Children::from_nodes(node.children.iter().map(|child| build_node(child, horizontal)));
    Handle::new(Node {
        style: style(node, parent_is_row),
        version: fresh_version(),
        children,
        layout: LayoutState::container(),
    })
}

/// Marks every node as needing layout; nodes are unique here, so `make_mut` never clones.
fn invalidate(node: &mut Handle<Node>) {
    let node = Handle::make_mut(node);
    node.layout.invalidate();
    for slot in node.children.make_mut() {
        invalidate(&mut slot.node);
    }
}

pub struct HookeTree {
    root: Handle<Node>,
    scratch: Scratch<f32>,
    constraints: RootConstraints<f32>,
}

impl HookeTree {
    pub fn new(template: &LayoutNode, width: f32, height: f32) -> Self {
        // The root has no parent. PanGUI's own translation treats it as the child of a column.
        Self {
            root: build_node(template, false),
            scratch: Scratch::new(),
            constraints: RootConstraints::viewport(width, height),
        }
    }

    pub fn mark_all_dirty(&mut self) {
        invalidate(&mut self.root);
    }

    pub fn compute_layout(&mut self) {
        let mut cx = LayoutCx::new(&mut self.scratch);
        hooke::layout(&mut self.root, self.constraints, &mut cx, &mut NoMeasure).expect("not cancelled");
        std::hint::black_box(&self.root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pangui_template::parse_test_case;
    use std::path::Path;
    use taffy::prelude::{auto, length, percent, Dimension, LengthPercentageAuto, NodeId, Rect, Size, TaffyTree};
    use taffy::style::Style as TaffyStyle;

    const DEFAULT_ROOT_SIZE: f32 = 1000.0;

    fn root_size(size: &Option<BenchmarkSize>) -> f32 {
        match size {
            Some(BenchmarkSize { mode: SizeMode::Pixels, value }) => *value,
            _ => DEFAULT_ROOT_SIZE,
        }
    }

    // A copy of `style()` in `benches/pangui.rs`.
    fn taffy_dimension(size: &Option<BenchmarkSize>) -> Dimension {
        match size {
            Some(BenchmarkSize { mode: SizeMode::Pixels, value }) => length(*value),
            Some(BenchmarkSize { mode: SizeMode::Percentage, value }) => percent(*value),
            Some(BenchmarkSize { mode: SizeMode::Fit | SizeMode::Expand | SizeMode::Ratio, .. }) | None => auto(),
        }
    }

    fn taffy_min_max(size: &Option<BenchmarkSize>, property: &str) -> LengthPercentageAuto {
        match size {
            Some(BenchmarkSize { mode: SizeMode::Pixels, value }) => length(*value),
            Some(BenchmarkSize { mode: SizeMode::Percentage, value }) => percent(*value),
            Some(BenchmarkSize { mode, .. }) => panic!("{mode:?} is not supported as a value of {property}"),
            None => auto(),
        }
    }

    fn taffy_style(node: &LayoutNode, parent_is_row: bool) -> TaffyStyle {
        use taffy::style::{AlignContent, AlignItems, Display, FlexDirection, FlexWrap, JustifyContent};
        let (main_size, main_min_size) =
            if parent_is_row { (&node.width, &node.min_width) } else { (&node.height, &node.min_height) };
        let expands = is_mode(main_size, SizeMode::Expand);
        let (flex_grow, flex_shrink, flex_basis) = match (main_size, main_min_size) {
            (Some(BenchmarkSize { mode: SizeMode::Expand, value }), _) => (*value, 1.0, length(0.0)),
            (_, Some(BenchmarkSize { mode: SizeMode::Expand, value })) => (*value, 0.0, auto()),
            _ => (0.0, 0.0, auto()),
        };
        let mut min_size = Size { width: auto(), height: auto() };
        if !(parent_is_row && is_mode(&node.min_width, SizeMode::Expand)) {
            min_size.width = taffy_min_max(&node.min_width, "MinWidth");
        }
        if !(!parent_is_row && is_mode(&node.min_height, SizeMode::Expand)) {
            min_size.height = taffy_min_max(&node.min_height, "MinHeight");
        }
        if expands {
            if parent_is_row && node.min_width.is_none() {
                min_size.width = length(0.0);
            }
            if !parent_is_row && node.min_height.is_none() {
                min_size.height = length(0.0);
            }
        }
        let aspect_ratio = match (&node.width, &node.height) {
            (Some(BenchmarkSize { mode: SizeMode::Ratio, value }), _) => Some(*value),
            (_, Some(BenchmarkSize { mode: SizeMode::Ratio, value })) => Some(1.0 / *value),
            _ => None,
        };
        let gap = length(node.gap.unwrap_or(0.0));
        TaffyStyle {
            display: Display::Flex,
            flex_direction: match node.flow_direction {
                Some(FlowDirection::Horizontal) => FlexDirection::Row,
                Some(FlowDirection::Vertical) | None => FlexDirection::Column,
            },
            flex_wrap: match node.wrap {
                Some(WrapMode::Auto) => FlexWrap::Wrap,
                Some(WrapMode::NoWrap) | None => FlexWrap::NoWrap,
            },
            size: Size { width: taffy_dimension(&node.width), height: taffy_dimension(&node.height) },
            min_size,
            max_size: Size {
                width: taffy_min_max(&node.max_width, "MaxWidth"),
                height: taffy_min_max(&node.max_height, "MaxHeight"),
            },
            aspect_ratio,
            gap: Size { width: gap, height: gap },
            padding: Rect {
                left: length(node.padding_left.unwrap_or(0.0)),
                right: length(node.padding_right.unwrap_or(0.0)),
                top: length(node.padding_top.unwrap_or(0.0)),
                bottom: length(node.padding_bottom.unwrap_or(0.0)),
            },
            margin: Rect {
                left: length(node.margin_left.unwrap_or(0.0)),
                right: length(node.margin_right.unwrap_or(0.0)),
                top: length(node.margin_top.unwrap_or(0.0)),
                bottom: length(node.margin_bottom.unwrap_or(0.0)),
            },
            flex_grow,
            flex_shrink,
            flex_basis,
            align_items: AlignItems::STRETCH,
            justify_content: JustifyContent::START,
            align_content: AlignContent::START,
            ..Default::default()
        }
    }

    fn build_taffy(tree: &mut TaffyTree, node: &LayoutNode, parent_is_row: bool) -> NodeId {
        let horizontal = node.flow_direction == Some(FlowDirection::Horizontal);
        let children: Vec<_> = node.children.iter().map(|child| build_taffy(tree, child, horizontal)).collect();
        tree.new_with_children(taffy_style(node, parent_is_row), &children).unwrap()
    }

    type Frame = [f32; 4];

    fn hooke_frames(node: &Handle<Node>, x: f32, y: f32, out: &mut Vec<Frame>) {
        let size = node.layout.size();
        out.push([x, y, size.width, size.height]);
        for slot in &node.children {
            hooke_frames(&slot.node, x + slot.offset.x, y + slot.offset.y, out);
        }
    }

    fn taffy_frames(tree: &TaffyTree, node: NodeId, x: f32, y: f32, out: &mut Vec<Frame>) {
        let layout = tree.layout(node).unwrap();
        let (x, y) = (x + layout.location.x, y + layout.location.y);
        out.push([x, y, layout.size.width, layout.size.height]);
        for child in tree.children(node).unwrap() {
            taffy_frames(tree, child, x, y, out);
        }
    }

    fn templates() -> Vec<(String, LayoutNode)> {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("pangui");
        let mut paths: Vec<_> = std::fs::read_dir(&directory).unwrap().map(|entry| entry.unwrap().path()).collect();
        paths.sort();
        paths
            .iter()
            .filter(|path| path.extension().is_some_and(|e| e == "txt"))
            .map(|path| {
                let file_name = path.file_name().unwrap().to_str().unwrap();
                let text = std::fs::read_to_string(path).unwrap();
                let case = parse_test_case(file_name, &text).unwrap_or_else(|error| panic!("{file_name}: {error}"));
                (case.name, case.root_node.unwrap())
            })
            .collect()
    }

    #[test]
    fn hooke_agrees_with_taffy_on_every_template() {
        let mut failures = Vec::new();
        for (name, root) in templates() {
            let (width, height) = (root_size(&root.width), root_size(&root.height));

            let mut hooke = HookeTree::new(&root, width, height);
            hooke.compute_layout();
            let mut ours = Vec::new();
            hooke_frames(&hooke.root, 0.0, 0.0, &mut ours);

            let mut tree = TaffyTree::new();
            tree.disable_rounding();
            let taffy_root = build_taffy(&mut tree, &root, false);
            tree.compute_layout(taffy_root, Size { width: Some(width).into(), height: Some(height).into() }).unwrap();
            let mut theirs = Vec::new();
            taffy_frames(&tree, taffy_root, 0.0, 0.0, &mut theirs);

            assert_eq!(ours.len(), theirs.len(), "{name}: node counts differ");
            let tolerance = if ours.len() >= 100_000 { 0.1 } else { 0.01 };
            let first = ours
                .iter()
                .zip(&theirs)
                .position(|(a, b)| a.iter().zip(b).any(|(a, b)| (a - b).abs() > tolerance));
            match first {
                None => println!("{name}: agree ({} nodes)", ours.len()),
                Some(i) => {
                    println!("{name}: node {i} differs: hooke {:?}, Taffy {:?}", ours[i], theirs[i]);
                    failures.push(name);
                }
            }
        }
        assert!(failures.is_empty(), "templates that differ: {failures:?}");
    }
}
