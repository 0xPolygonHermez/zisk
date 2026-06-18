use std::mem;

pub struct CodeLine {
    with_comments: bool,
    code: String,
    comment: String,
    comment_col: usize,
}

impl CodeLine {
    pub fn new(with_comments: bool, comment_col: usize) -> Self {
        Self { code: String::new(), comment: String::new(), with_comments, comment_col }
    }
    pub fn collect(&mut self) -> String {
        if self.with_comments {
            self.code.push_str(&format!("{: >1$}", "// ", self.comment_col - self.code.len() + 3));
            self.code.push_str(&self.comment);
            self.comment = String::new();
        }
        mem::take(&mut self.code)
    }
    pub fn append(&mut self, code: &str) {
        self.code.push_str(code);
        if !self.with_comments {
            return;
        }
        self.comment.push_str(code);
    }
    pub fn append_with_comments(&mut self, code: &str, comments: Option<&str>) {
        self.code.push_str(code);
        if !self.with_comments {
            return;
        }
        if let Some(comment) = comments {
            self.comment.push_str(comment);
        }
    }
}
