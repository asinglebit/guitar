#[rustfmt::skip]
use std::{
    cell::{
        RefCell
    },
    rc::Rc,
    collections::{
        HashMap
    },
};
#[rustfmt::skip]
use ratatui::{
    style::{
        Color
    }
};
#[rustfmt::skip]
use crate::{
    helpers::{
        colors::{
            ColorPicker
        }
    }
};

#[derive(Default)]
pub struct Stashes {
    pub colors: HashMap<u32, Color>,
}


impl Stashes {

    pub fn feed(
        &mut self,
        color: &Rc<RefCell<ColorPicker>>,
        stashes_lanes: &HashMap<u32, usize>,
    ) {

        // Initialize
        self.colors = HashMap::new();

        // Set tag colors
        for (oidi, &lane_idx) in stashes_lanes.iter() {
            self.colors.insert(*oidi, color.borrow().get_lane(lane_idx));
        }
    }
}
