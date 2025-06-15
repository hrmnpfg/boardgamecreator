use crate::Board;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::sync::mpsc::{Receiver, Sender};

macro_rules! try_eval {
  ($expr:expr, $board:expr, $context:expr, $depth:expr, $sender:expr, $receiver:expr, $is_evaluate_mate:expr) => {
    match $expr.evaluate(
      $board,
      $context,
      $depth,
      $sender,
      $receiver,
      $is_evaluate_mate,
    ) {
      RuleExpression::Err(e) => return RuleExpression::Err(e),
      val => val,
    }
  };
}

///zasada
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RuleExpression {
  Void,
  Boolean(bool),
  Integer(i32),
  Diff(Box<Self>, Box<Self>),
  ApiCall(String, Vec<String>),
  Variable(String),
  Pair(Box<Self>, Box<Self>),
  First(Box<Self>),
  Second(Box<Self>),
  And(Vec<Self>),
  Or(Vec<Self>),
  Not(Box<Self>),
  Equals(Box<Self>, Box<Self>),
  If(Box<Self>, Box<Self>, Box<Self>),
  Sum(Box<Self>, Box<Self>),

  Err(String),
}

impl RuleExpression {
  pub fn _new(s: String) -> Result<Self, Box<dyn Error>> {
    let rule: Self = serde_json::from_str(&s)?;
    Ok(rule)
  }

  pub fn list() -> HashMap<String, (u32, String)> {
    HashMap::from([
      ("Void".to_string(), (0, "empty rule".to_string())),
      (
        "Boolean".to_string(),
        (1, "boolean value\neg: Boolean false".to_string()),
      ),
      (
        "Integer".to_string(),
        (1, "integer value\neg: Integer 1".to_string()),
      ),
      (
        "Diff".to_string(),
        (2, "difference of two integers".to_string()),
      ),
      ("ApiCall".to_string(), (u32::MAX, "call api".to_string())),
      ("Variable".to_string(), (1, "create variable".to_string())),
      ("Pair".to_string(), (2, "create pair".to_string())),
      ("And".to_string(), (2, "and of two expressions".to_string())),
      ("Or".to_string(), (2, "or of two expressions".to_string())),
      ("Not".to_string(), (1, "not of expressions".to_string())),
      (
        "Equals".to_string(),
        (2, "checks if two values equal".to_string()),
      ),
      ("If".to_string(), (3, "if".to_string())),
      ("Sum".to_string(), (2, "sum of two expressions".to_string())),
      ("Error".to_string(), (1, "error".to_string())),
    ])
  }
  ///oblicza wyrażenie
  pub fn evaluate(
    &self,
    board: &mut Board,
    context: &mut HashMap<String, String>,
    depth: u32,
    sender: Option<&Sender<Option<String>>>,
    receiver: Option<&Receiver<String>>,
    is_evaluate_mate: bool,
  ) -> Self {
    // only calling api will increase depth since all the other can't really loop
    // so it is depth of apicall only
    if depth > 15 {
      return Self::Err("too many calls".to_string());
    }

    match self {
      Self::Equals(left, right) => {
        match (
          try_eval!(
            left,
            board,
            context,
            depth,
            sender,
            receiver,
            is_evaluate_mate
          ),
          try_eval!(
            right,
            board,
            context,
            depth,
            sender,
            receiver,
            is_evaluate_mate
          ),
        ) {
          (Self::Void, Self::Void) => Self::Boolean(true),
          (Self::Integer(a), Self::Integer(b)) => Self::Boolean(a == b),
          (Self::Boolean(a), Self::Boolean(b)) => Self::Boolean(a == b),
          (Self::Pair(a, b), Self::Pair(c, d)) => {
            let a1 = try_eval!(a, board, context, depth, sender, receiver, is_evaluate_mate);
            let b1 = try_eval!(b, board, context, depth, sender, receiver, is_evaluate_mate);
            let c1 = try_eval!(c, board, context, depth, sender, receiver, is_evaluate_mate);
            let d1 = try_eval!(d, board, context, depth, sender, receiver, is_evaluate_mate);

            let result = match (
              try_eval!(
                Self::Equals(Box::new(a1), Box::new(c1)),
                board,
                context,
                depth,
                sender,
                receiver,
                is_evaluate_mate
              ),
              try_eval!(
                Self::Equals(Box::new(b1), Box::new(d1)),
                board,
                context,
                depth,
                sender,
                receiver,
                is_evaluate_mate
              ),
            ) {
              (Self::Boolean(a), Self::Boolean(b)) => a && b,
              (a, b) => {
                return Self::Err(format!("Cannot compare different types: {:?}, {:?}", a, b));
              }
            };
            Self::Boolean(result)
          }
          (a, b) => Self::Err(format!("Cannot compare different types: {:?}, {:?}", a, b)),
        }
      }
      Self::Diff(a, b) => {
        match (
          try_eval!(a, board, context, depth, sender, receiver, is_evaluate_mate),
          try_eval!(b, board, context, depth, sender, receiver, is_evaluate_mate),
        ) {
          (Self::Integer(a1), Self::Integer(b1)) => Self::Integer((a1 - b1).abs()),
          (a, b) => Self::Err(format!(
            "Cannot calculate absolute difference for: {:?}, {:?}",
            a, b
          )),
        }
      }
      Self::ApiCall(api, arg) => board.call_api(
        api,
        arg.iter().map(|s| s.as_str()).collect(),
        context,
        depth + 1,
        sender,
        receiver,
        is_evaluate_mate,
      ),
      Self::Boolean(value) => Self::Boolean(*value),
      Self::Variable(name) => {
        let value = match context.get(name) {
          Some(v) => v,
          None => {
            return Self::Err(format!("No variable {} in context", name));
          }
        };
        if let Ok(int_val) = value.parse::<i32>() {
          return Self::Integer(int_val);
        }
        if let Ok(bool_val) = value.parse::<bool>() {
          return Self::Boolean(bool_val);
        }
        if let Some(a) = value.split_once(',') {
          let (p1, p2) = a;
          if let Ok(v1) = p1.parse::<i32>() {
            if let Ok(v2) = p2.parse::<i32>() {
              return Self::Pair(Box::new(Self::Integer(v1)), Box::new(Self::Integer(v2)));
            }
          }
        }
        Self::Variable(value.to_string())
      }
      Self::Integer(value) => Self::Integer(*value),
      Self::Void => Self::Void,
      Self::And(vec) => {
        let mut res = true;
        for item in vec {
          match try_eval!(
            item,
            board,
            context,
            depth,
            sender,
            receiver,
            is_evaluate_mate
          ) {
            Self::Boolean(a) => {
              res &= a;
              if !a {
                return Self::Boolean(false);
              }
            }
            a => {
              return Self::Err(format!("Not a boolean: {:?}", a));
            }
          }
        }
        Self::Boolean(res)
      }
      Self::Or(vec) => {
        let mut res = false;
        for item in vec {
          match try_eval!(
            item,
            board,
            context,
            depth,
            sender,
            receiver,
            is_evaluate_mate
          ) {
            Self::Boolean(a) => {
              res |= a;
              if a {
                return Self::Boolean(true);
              }
            }
            a => {
              return Self::Err(format!("Not a boolean: {:?}", a));
            }
          }
        }
        Self::Boolean(res)
      }
      Self::Not(inner) => {
        match try_eval!(
          inner,
          board,
          context,
          depth,
          sender,
          receiver,
          is_evaluate_mate
        ) {
          Self::Boolean(a) => Self::Boolean(!a),
          Self::Integer(0) => Self::Integer(1),
          Self::Integer(_) => Self::Integer(0),
          a => {
            return Self::Err(format!("Cannot calculate a negation for: {:?}", a));
          }
        }
      }
      Self::If(cond, t, f) => {
        match try_eval!(
          cond,
          board,
          context,
          depth,
          sender,
          receiver,
          is_evaluate_mate
        ) {
          Self::Boolean(true) => {
            try_eval!(t, board, context, depth, sender, receiver, is_evaluate_mate)
          }
          Self::Boolean(false) => {
            try_eval!(f, board, context, depth, sender, receiver, is_evaluate_mate)
          }
          a => {
            return Self::Err(format!("Not a boolean: {:?}", a));
          }
        }
      }
      Self::Pair(a, b) => Self::Pair(
        Box::new(try_eval!(
          a,
          board,
          context,
          depth,
          sender,
          receiver,
          is_evaluate_mate
        )),
        Box::new(try_eval!(
          b,
          board,
          context,
          depth,
          sender,
          receiver,
          is_evaluate_mate
        )),
      ),
      Self::First(inner) => {
        match try_eval!(
          inner,
          board,
          context,
          depth,
          sender,
          receiver,
          is_evaluate_mate
        ) {
          Self::Pair(a, _) => *a,
          a => {
            return Self::Err(format!("Cannot take first for: {:?}", a));
          }
        }
      }
      Self::Second(inner) => {
        match try_eval!(
          inner,
          board,
          context,
          depth,
          sender,
          receiver,
          is_evaluate_mate
        ) {
          Self::Pair(_, b) => *b,
          a => {
            return Self::Err(format!("Cannot take first for: {:?}", a));
          }
        }
      }
      Self::Sum(a, b) => {
        match (
          try_eval!(a, board, context, depth, sender, receiver, is_evaluate_mate),
          try_eval!(b, board, context, depth, sender, receiver, is_evaluate_mate),
        ) {
          (Self::Integer(a1), Self::Integer(b1)) => Self::Integer(a1 + b1),
          (Self::Pair(a1, b1), Self::Pair(c1, d1)) => {
            let a2 = try_eval!(
              a1,
              board,
              context,
              depth,
              sender,
              receiver,
              is_evaluate_mate
            );
            let b2 = try_eval!(
              b1,
              board,
              context,
              depth,
              sender,
              receiver,
              is_evaluate_mate
            );
            let c2 = try_eval!(
              c1,
              board,
              context,
              depth,
              sender,
              receiver,
              is_evaluate_mate
            );
            let d2 = try_eval!(
              d1,
              board,
              context,
              depth,
              sender,
              receiver,
              is_evaluate_mate
            );

            Self::Pair(
              Box::new(try_eval!(
                Self::Sum(Box::new(a2), Box::new(c2)),
                board,
                context,
                depth,
                sender,
                receiver,
                is_evaluate_mate
              )),
              Box::new(try_eval!(
                Self::Sum(Box::new(b2), Box::new(d2)),
                board,
                context,
                depth,
                sender,
                receiver,
                is_evaluate_mate
              )),
            )
          }
          (a, b) => Self::Err(format!("Cannot calculate sum for: {:?}, {:?}", a, b)),
        }
      }
      Self::Err(a) => Self::Err(a.clone()),
    }
  }

  pub fn string2(self) -> String {
    match self {
      Self::Pair(a, b) => format!("{},{}", a.string2(), b.string2()),
      Self::Void => "Void".to_string(),
      Self::Boolean(a) => format!("{}", a),
      Self::Integer(a) => format!("{}", a),
      Self::Diff(a, b) => format!("|{}-{}|", a.string2(), b.string2()),
      Self::ApiCall(name, args) => {
        let args_str = args.join(", ");
        format!("{}({})", name, args_str)
      }
      Self::Variable(name) => name.to_string(),
      Self::First(a) => format!("first({})", a.string2()),
      Self::Second(a) => format!("second({})", a.string2()),
      Self::And(exprs) => {
        let parts: Vec<String> = exprs.into_iter().map(|e| e.string2()).collect();
        format!("and({})", parts.join(", "))
      }
      Self::Or(exprs) => {
        let parts: Vec<String> = exprs.into_iter().map(|e| e.string2()).collect();
        format!("or({})", parts.join(", "))
      }
      Self::Not(a) => format!("not({})", a.string2()),
      Self::Equals(a, b) => format!("{} == {}", a.string2(), b.string2()),
      Self::If(a, b, c) => format!(
        "if {} then {} else {}",
        a.string2(),
        b.string2(),
        c.string2()
      ),
      Self::Sum(a, b) => format!("{} + {}", a.string2(), b.string2()),
      Self::Err(msg) => format!("Error: {}", msg),
    }
  }
}
