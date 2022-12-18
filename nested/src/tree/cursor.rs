use {
    crate::list::ListCursorMode,
    cgmath::Vector2
};

//<<<<>>>><<>><><<>><<<*>>><<>><><<>><<<<>>>>

#[derive(Clone, Eq, PartialEq)]
pub struct TreeCursor {
    pub leaf_mode: ListCursorMode,
    pub tree_addr: Vec<isize>,
}

impl TreeCursor {
    pub fn home() -> Self {
        TreeCursor {
            leaf_mode: ListCursorMode::Insert,
            tree_addr: vec![0]
        }
    }

    pub fn none() -> Self {
        TreeCursor {
            leaf_mode: ListCursorMode::Select,
            tree_addr: vec![],
        }
    }
}

impl Default for TreeCursor {
    fn default() -> Self {
        TreeCursor::none()
    }
}
