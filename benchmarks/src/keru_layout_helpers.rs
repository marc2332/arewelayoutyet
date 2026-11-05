//! Adapter for `keru_layout`.
use keru_layout::{Arrange, Axis, ChildrenLayout, LayoutData, LayoutParams, NodeLayoutData, Pos, Size, TreeAdapter, Xy, XyRect};

use crate::pangui_template::{BenchmarkSize, FlowDirection, LayoutNode, SizeMode, WrapMode};

fn size(size: &Option<BenchmarkSize>, axis: Axis) -> Size {
    match size {
        None | Some(BenchmarkSize { mode: SizeMode::Fit, .. }) => Size::FitContent,
        Some(BenchmarkSize { mode: SizeMode::Pixels, value }) => Size::Pixels(*value),
        Some(BenchmarkSize { mode: SizeMode::Percentage, value }) => Size::Frac(*value),
        Some(BenchmarkSize { mode: SizeMode::Expand, .. }) => Size::Fill,
        Some(BenchmarkSize { mode: SizeMode::Ratio, value }) => match axis {
            Axis::X => Size::AspectRatio(*value),
            Axis::Y => Size::AspectRatio(1.0 / *value),
        },
    }
}

fn bound(bound: &Option<BenchmarkSize>) -> Option<Option<Size>> {
    Some(match bound {
        None => None,
        Some(BenchmarkSize { mode: SizeMode::Pixels, value }) => Some(Size::Pixels(*value)),
        Some(BenchmarkSize { mode: SizeMode::Percentage, value }) => Some(Size::Frac(*value)),
        Some(BenchmarkSize { mode: SizeMode::Expand, .. }) => Some(Size::Fill),
        Some(_) => return None,
    })
}

fn expand_weight(size: &Option<BenchmarkSize>) -> Option<f32> {
    match size {
        Some(BenchmarkSize { mode: SizeMode::Expand, value }) => Some(*value),
        _ => None,
    }
}

fn params(node: &LayoutNode, parent_axis: Option<Axis>) -> Option<LayoutParams> {
    let margins = [node.margin_left, node.margin_right, node.margin_top, node.margin_bottom];
    if margins.iter().any(|margin| margin.unwrap_or(0.0) != 0.0) || node.wrap == Some(WrapMode::Auto) {
        return None;
    }
    let (left, right) = (node.padding_left.unwrap_or(0.0), node.padding_right.unwrap_or(0.0));
    let (top, bottom) = (node.padding_top.unwrap_or(0.0), node.padding_bottom.unwrap_or(0.0));
    let axis = stack_axis(node);
    // keru_layout has one weight per node, and it only has an effect on the stack axis of the parent.
    let (main, min, max) = match parent_axis {
        Some(Axis::X) => (&node.width, &node.min_width, &node.max_width),
        Some(Axis::Y) => (&node.height, &node.min_height, &node.max_height),
        None => (&None, &None, &None),
    };
    let fill_weight = expand_weight(main).or(expand_weight(max)).or(expand_weight(min)).unwrap_or(1.0);
    Some(LayoutParams {
        size: Xy::new(size(&node.width, Axis::X), size(&node.height, Axis::Y)),
        min_size: Xy::new(bound(&node.min_width)?, bound(&node.min_height)?),
        max_size: Xy::new(bound(&node.max_width)?, bound(&node.max_height)?),
        fill_weight,
        padding: Xy::new([left, right], [top, bottom]),
        children_layout: ChildrenLayout::Stack { arrange: Arrange::Start, axis, spacing: node.gap.unwrap_or(0.0) },
        position: Xy::new(Pos::Start, Pos::Start),
        ..LayoutParams::DEFAULT
    })
}

fn stack_axis(node: &LayoutNode) -> Axis {
    match node.flow_direction {
        Some(FlowDirection::Horizontal) => Axis::X,
        Some(FlowDirection::Vertical) | None => Axis::Y,
    }
}

fn count(node: &LayoutNode) -> usize {
    1 + node.children.iter().map(count).sum::<usize>()
}

struct Node {
    parent: u32,
    first_child: Option<u32>,
    last_child: Option<u32>,
    next_sibling: Option<u32>,
    params: LayoutParams,
    layout: NodeLayoutData,
    rect: XyRect,
}

struct Tree {
    nodes: Vec<Node>,
    layout_data: LayoutData<u32>,
    window_size: Xy<f32>,
}

impl Tree {
    fn with_capacity(capacity: usize, window_size: Xy<f32>) -> Self {
        let mut tree = Tree { nodes: Vec::with_capacity(capacity), layout_data: LayoutData::new(), window_size };
        tree.push(0, LayoutParams::DEFAULT);
        tree
    }

    fn push(&mut self, parent: u32, params: LayoutParams) -> u32 {
        let i = self.nodes.len() as u32;
        let rect = Xy::new([0.0, 0.0], [0.0, 0.0]);
        self.nodes.push(Node { parent, first_child: None, last_child: None, next_sibling: None, params, layout: NodeLayoutData::new(), rect });
        i
    }

    fn add(&mut self, parent: u32, params: LayoutParams) -> u32 {
        let i = self.push(parent, params);
        match self.nodes[parent as usize].last_child {
            Some(last) => self.nodes[last as usize].next_sibling = Some(i),
            None => self.nodes[parent as usize].first_child = Some(i),
        }
        self.nodes[parent as usize].last_child = Some(i);
        i
    }
}

impl TreeAdapter for Tree {
    type NodeId = u32;

    fn root(&self) -> u32 {
        0
    }

    fn parent(&self, i: u32) -> Option<u32> {
        (i != 0).then(|| self.nodes[i as usize].parent)
    }

    fn first_child(&self, i: u32) -> Option<u32> {
        self.nodes[i as usize].first_child
    }

    fn next_sibling(&self, i: u32) -> Option<u32> {
        self.nodes[i as usize].next_sibling
    }

    fn node_layout_params(&self, i: u32) -> &LayoutParams {
        &self.nodes[i as usize].params
    }

    fn node_layout_data(&self, i: u32) -> &NodeLayoutData {
        &self.nodes[i as usize].layout
    }

    fn node_layout_data_mut(&mut self, i: u32) -> &mut NodeLayoutData {
        &mut self.nodes[i as usize].layout
    }

    fn layout_data(&mut self) -> &mut LayoutData<u32> {
        &mut self.layout_data
    }

    fn measure_text_width(&mut self, _i: u32) -> f32 {
        0.0
    }

    fn measure_text_height(&mut self, _i: u32, _wrap_width: Option<f32>) -> f32 {
        0.0
    }

    fn write_output_rect(&mut self, i: u32, rect: XyRect, _local_rect: XyRect) {
        self.nodes[i as usize].rect = rect;
    }
}

fn add(tree: &mut Tree, parent: u32, node: &LayoutNode, parent_axis: Option<Axis>) -> Option<()> {
    let i = tree.add(parent, params(node, parent_axis)?);
    for child in &node.children {
        add(tree, i, child, Some(stack_axis(node)))?;
    }
    Some(())
}

pub struct KeruLayoutTree {
    tree: Tree,
}

impl KeruLayoutTree {
    pub fn new(template: &LayoutNode, width: f32, height: f32) -> Option<Self> {
        let mut tree = Tree::with_capacity(count(template) + 1, Xy::new(width, height));
        add(&mut tree, 0, template, None)?;
        Some(Self { tree })
    }

    /// `solve_layout` is always a full solve, so there is nothing to invalidate.
    pub fn mark_all_dirty(&mut self) {}

    pub fn compute_layout(&mut self) {
        let window_size = self.tree.window_size;
        self.tree.solve_layout(window_size);
        std::hint::black_box(&self.tree.nodes);
    }
}
