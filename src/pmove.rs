use serde::{Deserialize, Serialize};
use crate::RuleExpression;
use std::error::Error;

///move structure, condition is for eval and consequence for execute
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PMove{
    pub condition:Box<RuleExpression>,
    pub consequences:Option<Vec<Box<RuleExpression>>>
}

impl PMove{
    pub fn new(s: String) -> Result<Self, Box<dyn Error>> {
        let mov: PMove =  serde_json::from_str(&s)?;
        Ok(mov)
    }

    pub fn default_move() -> Self{
        PMove { condition: Box::new(RuleExpression::Boolean(false)), consequences: None }
    }
}
