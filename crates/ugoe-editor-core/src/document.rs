use ugoe_model::Model;

use crate::Command;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum EditError {
    #[error("parameter `{0}` already exists")]
    DuplicateParameter(String),
    #[error("parameter `{0}` not found")]
    ParameterNotFound(String),
    #[error("invalid range: min {min} <= default {default} <= max {max} must hold")]
    InvalidRange { min: f32, default: f32, max: f32 },
}

/// 編集中のモデルと履歴。
///
/// 暫定実装としてスナップショット方式の Undo を使う (`docs/adr/0005-editor-command-model.md`)。
#[derive(Debug)]
pub struct Document {
    model: Model,
    undo: Vec<Model>,
    redo: Vec<Model>,
}

impl Document {
    pub fn new(model: Model) -> Self {
        Self {
            model,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    /// コマンドを適用する。失敗時はモデルを変更しない。
    pub fn apply(&mut self, command: Command) -> Result<(), EditError> {
        let mut next = self.model.clone();
        apply_to(&mut next, command)?;
        self.undo.push(std::mem::replace(&mut self.model, next));
        self.redo.clear();
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(std::mem::replace(&mut self.model, prev));
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(std::mem::replace(&mut self.model, next));
                true
            }
            None => false,
        }
    }
}

fn apply_to(model: &mut Model, command: Command) -> Result<(), EditError> {
    match command {
        Command::AddParameter { parameter } => {
            if model.parameters.iter().any(|p| p.id == parameter.id) {
                return Err(EditError::DuplicateParameter(parameter.id.to_string()));
            }
            model.parameters.push(parameter);
        }
        Command::RemoveParameter { id } => {
            let before = model.parameters.len();
            model.parameters.retain(|p| p.id != id);
            if model.parameters.len() == before {
                return Err(EditError::ParameterNotFound(id.to_string()));
            }
            // TODO(T0601): このパラメータを参照するキーフォーム軸の除去。
        }
        Command::SetParameterRange {
            id,
            min,
            default,
            max,
        } => {
            if !(min <= default && default <= max) {
                return Err(EditError::InvalidRange { min, default, max });
            }
            let p = model
                .parameters
                .iter_mut()
                .find(|p| p.id == id)
                .ok_or_else(|| EditError::ParameterNotFound(id.to_string()))?;
            (p.min, p.default, p.max) = (min, default, max);
        }
        Command::Batch { commands, .. } => {
            for c in commands {
                apply_to(model, c)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use ugoe_model::fixtures;

    use super::*;

    #[test]
    fn apply_undo_redo() {
        let mut doc = Document::new(fixtures::single_quad());
        let cmd: Command = serde_json::from_str(
            r#"{"op":"set_parameter_range","id":"X","min":-2,"default":0,"max":2}"#,
        )
        .unwrap();
        doc.apply(cmd).unwrap();
        assert_eq!(doc.model().parameters[0].max, 2.0);
        assert!(doc.undo());
        assert_eq!(doc.model().parameters[0].max, 1.0);
        assert!(doc.redo());
        assert_eq!(doc.model().parameters[0].max, 2.0);
    }

    #[test]
    fn failed_command_leaves_model_untouched() {
        let mut doc = Document::new(fixtures::single_quad());
        let before = doc.model().clone();
        let err = doc.apply(Command::RemoveParameter { id: "Nope".into() });
        assert_eq!(err, Err(EditError::ParameterNotFound("Nope".into())));
        assert_eq!(doc.model(), &before);
        assert!(!doc.undo());
    }
}
