#![allow(clippy::too_many_arguments)]
use num_format::{Locale, ToFormattedString};

pub struct StatsReport {
    pub output: String,
    pub cost_divisor: f64,
    pub step_divisor: f64,
    pub identation: String,
    pub label_width: usize,
    pub short_label_width: usize,
    pub label_width_stack: Vec<usize>,
}
impl Default for StatsReport {
    fn default() -> Self {
        Self::new()
    }
}

impl StatsReport {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            cost_divisor: 1.0,
            step_divisor: 1.0,
            identation: String::new(),
            label_width: 24,
            short_label_width: 10,
            label_width_stack: Vec::new(),
        }
    }

    pub fn set_total_cost(&mut self, value: u64) {
        self.cost_divisor = value as f64 / 100.0;
    }
    pub fn set_steps(&mut self, value: u64) {
        self.step_divisor = value as f64 / 100.0;
    }
    pub fn set_identation(&mut self, level: usize) {
        self.identation = format!("|{}", " ".repeat(level * 4));
    }
    pub fn set_label_width(&mut self, width: usize) {
        self.label_width = width;
    }
    pub fn set_and_push_label_width(&mut self, width: usize) {
        self.push_label_width();
        self.label_width = width;
    }
    pub fn push_label_width(&mut self) {
        self.label_width_stack.push(self.label_width);
    }
    pub fn pop_label_width(&mut self) {
        if let Some(width) = self.label_width_stack.pop() {
            self.label_width = width;
        }
    }

    pub fn add(&mut self, text: &str) {
        self.output += text;
    }
    pub fn add_cost(&mut self, label: &str, cost: u64) {
        self.output += &format!(
            "{}{:<label_width$} {:>15}\n",
            self.identation,
            label,
            cost.to_formatted_string(&Locale::en),
            label_width = self.label_width
        );
    }
    pub fn title(&mut self, label: &str) {
        self.output += &format!("\n{}{label}\n{}\n", self.identation, "-".repeat(label.len()));
    }

    fn line_from_title(&mut self, title: &str) {
        self.output += &format!(
            "\n{identation}{title}\n{identation}{}\n",
            &"-".repeat(title.len()),
            identation = self.identation,
        );
    }
    pub fn title_cost(&mut self, label: &str, cost_label: &str) {
        self.line_from_title(&format!(
            "{:<label_width$} {:>15}",
            label,
            cost_label,
            label_width = self.label_width
        ));
    }

    pub fn title_cost_perc(&mut self, label: &str, cost_label: &str) {
        self.line_from_title(&format!(
            "{label:<label_width$} {cost_label:>15}       %",
            label_width = self.label_width
        ));
    }

    pub fn add_perc(&mut self, label: &str, cost: u64, total: u64) {
        self.output += &format!(
            "{}{label:<label_width$} {:>15} {:6.2}%\n",
            self.identation,
            cost.to_formatted_string(&Locale::en),
            (cost as f64 * 100.0) / total as f64,
            label_width = self.label_width,
        );
    }

    pub fn add_cost_perc(&mut self, label: &str, cost: u64) {
        self.output += &format!(
            "{}{label:<label_width$} {:>15} {:6.2}%\n",
            self.identation,
            cost.to_formatted_string(&Locale::en),
            cost as f64 / self.cost_divisor,
            label_width = self.label_width,
        );
    }

    pub fn ln(&mut self) {
        self.output += "\n";
    }

    pub fn title_top_perc(&mut self, title: &str) {
        self.output += &format!(
            "\n{identation}{title}\n{identation}{}\n",
            &"-".repeat(std::cmp::min(title.len(), 22)),
            identation = self.identation,
        );
    }

    pub fn title_autowidth(&mut self, title: &str) {
        self.output += &format!(
            "\n{identation}{title}\n{identation}{}\n",
            &"-".repeat(title.len()),
            identation = self.identation,
        );
    }

    pub fn add_top_cost_perc(&mut self, label: &str, cost: u64) {
        self.output += &format!(
            "{}{:>15} {:6.2}% {label}\n",
            self.identation,
            cost.to_formatted_string(&Locale::en),
            cost as f64 / self.cost_divisor
        );
    }

    pub fn add_top_cost_depth_perc(&mut self, label: &str, cost: u64, depth: Option<usize>) {
        if let Some(depth) = depth {
            self.output += &format!(
                "{}{:>15} {:6.2}% {depth:2} {label}\n",
                self.identation,
                cost.to_formatted_string(&Locale::en),
                cost as f64 / self.cost_divisor
            );
            return;
        }
        self.output += &format!(
            "{}{:>15} {:6.2}%    {label}\n",
            self.identation,
            cost.to_formatted_string(&Locale::en),
            cost as f64 / self.cost_divisor
        );
    }

    pub fn add_top_cost_calls_perc(&mut self, label: &str, cost: u64, calls: usize) {
        self.output += &format!(
            "{}{:>15} {:6.2}% {:>10} {label}\n",
            self.identation,
            cost.to_formatted_string(&Locale::en),
            cost as f64 / self.cost_divisor,
            calls.to_formatted_string(&Locale::en)
        );
    }

    pub fn add_top_step_calls_perc(&mut self, label: &str, steps: u64, calls: usize) {
        self.output += &format!(
            "{}{:>15} {:6.2}% {:>10} {label}\n",
            self.identation,
            steps.to_formatted_string(&Locale::en),
            steps as f64 / self.step_divisor,
            calls.to_formatted_string(&Locale::en)
        );
    }

    pub fn add_top_step_perc(&mut self, label: &str, cost: u64) {
        self.output += &format!(
            "{}{:>15} {:6.2}% {label}\n",
            self.identation,
            cost.to_formatted_string(&Locale::en),
            cost as f64 / self.step_divisor
        );
    }

    pub fn add_top_step_depth_perc(&mut self, label: &str, cost: u64, depth: Option<usize>) {
        if let Some(depth) = depth {
            self.output += &format!(
                "{}{:>15} {:6.2}% {depth:2} {label}\n",
                self.identation,
                cost.to_formatted_string(&Locale::en),
                cost as f64 / self.step_divisor
            );
            return;
        }
        self.output += &format!(
            "{}{:>15} {:6.2}%    {label}\n",
            self.identation,
            cost.to_formatted_string(&Locale::en),
            cost as f64 / self.step_divisor
        );
    }

    pub fn title_top_count_perc(&mut self, title: &str) {
        self.output += &format!(
            "\n{identation}{title}\n{identation}{}\n",
            &"-".repeat(std::cmp::min(title.len(), 38)),
            identation = self.identation,
        );
    }

    pub fn add_top_count_step_perc(&mut self, label: &str, count: u64, step: u64) {
        self.output += &format!(
            "{}{:>15} {:>15} {:6.2}% {label}\n",
            self.identation,
            count.to_formatted_string(&Locale::en),
            step.to_formatted_string(&Locale::en),
            step as f64 / self.step_divisor
        );
    }

    pub fn title_count_cost_perc(
        &mut self,
        label: &str,
        count_label: &str,
        cost_label: &str,
        comment: &str,
    ) {
        self.line_from_title(&format!(
            "{label:<label_width$} {count_label:>15} {cost_label:>15}       %{comment}",
            label_width = self.label_width,
        ));
    }

    pub fn add_count_cost_perc(&mut self, label: &str, count: u64, cost: u64, comment: &str) {
        self.output += &format!(
            "{}{:<label_width$} {:>15} {:>15} {:6.2}%{comment}\n",
            self.identation,
            label,
            count.to_formatted_string(&Locale::en),
            cost.to_formatted_string(&Locale::en),
            cost as f64 / self.cost_divisor,
            label_width = self.label_width,
        );
    }

    pub fn title_count_perc_cost_perc(
        &mut self,
        label: &str,
        count_label: &str,
        perc_label: &str,
        cost_label: &str,
        comment: &str,
    ) {
        self.line_from_title(&format!(
            "{label:<label_width$} {count_label:>15} {perc_label:>6} {cost_label:>15}       %{comment}",
            label_width = self.label_width,
        ));
    }

    pub fn add_count_perc_cost_perc(
        &mut self,
        label: &str,
        count: u64,
        perc: f64,
        cost: u64,
        comment: &str,
    ) {
        self.output += &format!(
            "{}{:<label_width$} {:>15} {:6.2}% {:>15} {:6.2}%{comment}\n",
            self.identation,
            label,
            count.to_formatted_string(&Locale::en),
            perc,
            cost.to_formatted_string(&Locale::en),
            cost as f64 / self.cost_divisor,
            label_width = self.label_width,
        );
    }

    pub fn title_step_cost_detail_cost(
        &mut self,
        label: &str,
        index_label: &str,
        count_label: &str,
        step_label: &str,
        cost_label: &str,
        cost_main_label: &str,
        cost_ops_label: &str,
        cost_precomp_label: &str,
        cost_mem_label: &str,
    ) {
        self.output += &format!(
            "{}{label:<label_width$} {index_label:>10} {count_label:>10} {step_label:>15}       % {cost_label:>15}       % {cost_main_label:>15} {cost_ops_label:>15} {cost_precomp_label:>15} {cost_mem_label:>15}\n",
            self.identation,
            label_width = self.label_width,
        );
    }

    pub fn add_step_cost_detail_cost(
        &mut self,
        label: &str,
        index: usize,
        count: u64,
        step: u64,
        cost: u64,
        cost_main: u64,
        cost_ops: u64,
        cost_precomp: u64,
        cost_mem: u64,
        comment: &str,
    ) {
        self.output += &format!(
            "{}{:<label_width$} {:>10} {:>10} {:>15} {:6.2}% {:>15} {:6.2}% {:>15} {:>15} {:>15} {:>15}{comment}\n",
            self.identation,
            label,
            index.to_formatted_string(&Locale::en),
            count.to_formatted_string(&Locale::en),
            step.to_formatted_string(&Locale::en),
            step as f64 / self.step_divisor,
            cost.to_formatted_string(&Locale::en),
            cost as f64 / self.cost_divisor,
            cost_main.to_formatted_string(&Locale::en),
            cost_ops.to_formatted_string(&Locale::en),
            cost_precomp.to_formatted_string(&Locale::en),
            cost_mem.to_formatted_string(&Locale::en),
            label_width = self.label_width,
        );
    }

    pub fn add_separator(&mut self) {
        self.output += &format!(
            "{}------------------------------------------------------------\n",
            self.identation
        );
    }
    pub fn add_separator_width(&mut self, width: usize) {
        self.output += &format!("{}{:-<width$}\n", self.identation, "");
    }
}
