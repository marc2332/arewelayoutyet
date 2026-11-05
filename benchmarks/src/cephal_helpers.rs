use cephal::style::{
    AlignContent, AlignItems, Dimension, Display, FlexDirection, FlexWrap, LengthPercentage,
    LengthPercentageAuto,
};
use cephal::{AvailableSpace, NodeId, Rect, Size, Style, Tree};

use crate::pangui_template::{BenchmarkSize, FlowDirection, LayoutNode, SizeMode, WrapMode};

fn is_mode(size: &Option<BenchmarkSize>, mode: SizeMode) -> bool {
    size.is_some_and(|size| size.mode == mode)
}

fn dimension(size: &Option<BenchmarkSize>) -> Dimension {
    match size {
        Some(BenchmarkSize {
            mode: SizeMode::Pixels,
            value,
        }) => Dimension::length(*value),
        Some(BenchmarkSize {
            mode: SizeMode::Percentage,
            value,
        }) => Dimension::percent(*value),
        _ => Dimension::AUTO,
    }
}

fn min_max_size(size: &Option<BenchmarkSize>) -> LengthPercentageAuto {
    match size {
        Some(BenchmarkSize {
            mode: SizeMode::Pixels,
            value,
        }) => LengthPercentageAuto::length(*value),
        Some(BenchmarkSize {
            mode: SizeMode::Percentage,
            value,
        }) => LengthPercentageAuto::percent(*value),
        Some(BenchmarkSize { mode, .. }) => panic!("{mode:?} is not supported as a size bound"),
        None => LengthPercentageAuto::AUTO,
    }
}

fn style(node: &LayoutNode, parent_is_row: bool) -> Style {
    let (main_size, main_minimum) = if parent_is_row {
        (&node.width, &node.min_width)
    } else {
        (&node.height, &node.min_height)
    };
    let (flex_grow, flex_shrink, flex_basis) = match (main_size, main_minimum) {
        (
            Some(BenchmarkSize {
                mode: SizeMode::Expand,
                value,
            }),
            _,
        ) => (*value, 1.0, Dimension::ZERO),
        (
            _,
            Some(BenchmarkSize {
                mode: SizeMode::Expand,
                value,
            }),
        ) => (*value, 0.0, Dimension::AUTO),
        _ => (0.0, 0.0, Dimension::AUTO),
    };
    let mut min_size = Size {
        width: LengthPercentageAuto::AUTO,
        height: LengthPercentageAuto::AUTO,
    };
    if !(parent_is_row && is_mode(&node.min_width, SizeMode::Expand)) {
        min_size.width = min_max_size(&node.min_width);
    }
    if !(!parent_is_row && is_mode(&node.min_height, SizeMode::Expand)) {
        min_size.height = min_max_size(&node.min_height);
    }
    if is_mode(main_size, SizeMode::Expand) {
        if parent_is_row && node.min_width.is_none() {
            min_size.width = LengthPercentageAuto::ZERO;
        }
        if !parent_is_row && node.min_height.is_none() {
            min_size.height = LengthPercentageAuto::ZERO;
        }
    }
    let aspect_ratio = match (&node.width, &node.height) {
        (
            Some(BenchmarkSize {
                mode: SizeMode::Ratio,
                value,
            }),
            _,
        ) => Some(*value),
        (
            _,
            Some(BenchmarkSize {
                mode: SizeMode::Ratio,
                value,
            }),
        ) => Some(1.0 / *value),
        _ => None,
    };
    let gap = LengthPercentage::length(node.gap.unwrap_or(0.0));
    Style {
        display: Display::Flex,
        flex_direction: match node.flow_direction {
            Some(FlowDirection::Horizontal) => FlexDirection::Row,
            _ => FlexDirection::Column,
        },
        flex_wrap: match node.wrap {
            Some(WrapMode::Auto) => FlexWrap::Wrap,
            _ => FlexWrap::NoWrap,
        },
        size: Size {
            width: dimension(&node.width),
            height: dimension(&node.height),
        },
        min_size,
        max_size: Size {
            width: min_max_size(&node.max_width),
            height: min_max_size(&node.max_height),
        },
        aspect_ratio,
        gap: Size {
            width: gap,
            height: gap,
        },
        padding: Rect {
            left: LengthPercentage::length(node.padding_left.unwrap_or(0.0)),
            right: LengthPercentage::length(node.padding_right.unwrap_or(0.0)),
            top: LengthPercentage::length(node.padding_top.unwrap_or(0.0)),
            bottom: LengthPercentage::length(node.padding_bottom.unwrap_or(0.0)),
        },
        margin: Rect {
            left: LengthPercentageAuto::length(node.margin_left.unwrap_or(0.0)),
            right: LengthPercentageAuto::length(node.margin_right.unwrap_or(0.0)),
            top: LengthPercentageAuto::length(node.margin_top.unwrap_or(0.0)),
            bottom: LengthPercentageAuto::length(node.margin_bottom.unwrap_or(0.0)),
        },
        flex_grow,
        flex_shrink,
        flex_basis,
        align_items: Some(AlignItems::STRETCH),
        justify_content: Some(AlignContent::START),
        align_content: Some(AlignContent::START),
        ..Style::DEFAULT
    }
}

fn build_node(tree: &mut Tree<()>, node: &LayoutNode, parent_is_row: bool) -> NodeId {
    let horizontal = node.flow_direction == Some(FlowDirection::Horizontal);
    let children: Vec<_> = node
        .children
        .iter()
        .map(|child| build_node(tree, child, horizontal))
        .collect();
    tree.new_with_children(style(node, parent_is_row), &children)
}

pub struct CephalTree {
    tree: Tree<()>,
    root: NodeId,
    available_space: Size<AvailableSpace>,
}

impl CephalTree {
    pub fn new(template: &LayoutNode, width: f32, height: f32) -> Self {
        let mut tree = Tree::new();
        let root = build_node(&mut tree, template, false);
        Self {
            tree,
            root,
            available_space: Size {
                width: AvailableSpace::Definite(width),
                height: AvailableSpace::Definite(height),
            },
        }
    }

    pub fn mark_all_dirty(&mut self) {
        self.tree.clear_layout();
    }

    pub fn compute_layout(&mut self) {
        self.tree.compute_layout(self.root, self.available_space);
        std::hint::black_box(&self.tree);
    }
}
