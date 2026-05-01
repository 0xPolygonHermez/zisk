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
    pub use_thousands_sep: bool,
    pub sdk_width: usize,
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
            use_thousands_sep: true,
            sdk_width: 120,
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

    fn format_number(&self, num: u64) -> String {
        if self.use_thousands_sep {
            num.to_formatted_string(&Locale::en)
        } else {
            num.to_string()
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
            self.format_number(cost),
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
            self.format_number(cost),
            (cost as f64 * 100.0) / total as f64,
            label_width = self.label_width,
        );
    }

    pub fn add_cost_perc(&mut self, label: &str, cost: u64) {
        self.output += &format!(
            "{}{label:<label_width$} {:>15} {:6.2}%\n",
            self.identation,
            self.format_number(cost),
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

    pub fn title_auto_width(&mut self, title: &str) {
        self.output += &format!(
            "\n{identation}{title}\n{identation}{}\n",
            &"-".repeat(title.len()),
            identation = self.identation,
        );
    }

    pub fn title_fixed_width(&mut self, title: &str, width: usize) {
        self.output += &format!(
            "\n{identation}{title}\n{identation}{}\n",
            &"-".repeat(width),
            identation = self.identation,
        );
    }

    pub fn add_top_cost_perc(&mut self, label: &str, cost: u64) {
        self.output += &format!(
            "{}{:>15} {:6.2}% {label}\n",
            self.identation,
            self.format_number(cost),
            cost as f64 / self.cost_divisor
        );
    }

    pub fn add_top_cost_depth_perc(&mut self, label: &str, cost: u64, depth: Option<usize>) {
        if let Some(depth) = depth {
            self.output += &format!(
                "{}{:>15} {:6.2}% {depth:2} {label}\n",
                self.identation,
                self.format_number(cost),
                cost as f64 / self.cost_divisor
            );
            return;
        }
        self.output += &format!(
            "{}{:>15} {:6.2}%    {label}\n",
            self.identation,
            self.format_number(cost),
            cost as f64 / self.cost_divisor
        );
    }

    pub fn add_top_calls_perc(&mut self, label: &str, value: u64, calls: usize, total: f64) {
        self.output += &format!(
            "{}{:>15} {:6.2}% {:>10} {:>15} {label}\n",
            self.identation,
            self.format_number(value),
            value as f64 / total,
            self.format_number(calls as u64),
            self.format_number(value / calls as u64)
        );
    }

    pub fn add_top_cost_calls_perc(&mut self, label: &str, cost: u64, calls: usize) {
        self.add_top_calls_perc(label, cost, calls, self.cost_divisor)
    }

    pub fn add_top_step_calls_perc(&mut self, label: &str, steps: u64, calls: usize) {
        self.add_top_calls_perc(label, steps, calls, self.step_divisor)
    }

    pub fn add_top_step_perc(&mut self, label: &str, cost: u64) {
        self.output += &format!(
            "{}{:>15} {:6.2}% {label}\n",
            self.identation,
            self.format_number(cost),
            cost as f64 / self.step_divisor
        );
    }

    pub fn add_top_step_depth_perc(&mut self, label: &str, cost: u64, depth: Option<usize>) {
        if let Some(depth) = depth {
            self.output += &format!(
                "{}{:>15} {:6.2}% {depth:2} {label}\n",
                self.identation,
                self.format_number(cost),
                cost as f64 / self.step_divisor
            );
            return;
        }
        self.output += &format!(
            "{}{:>15} {:6.2}%    {label}\n",
            self.identation,
            self.format_number(cost),
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
            self.format_number(count),
            self.format_number(step),
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
            self.format_number(count),
            self.format_number(cost),
            cost as f64 / self.cost_divisor,
            label_width = self.label_width,
        );
    }

    pub fn title_count_cost_perc2(
        &mut self,
        label: &str,
        count_label: &str,
        cost_label: &str,
        comment: &str,
    ) {
        self.line_from_title(&format!(
            "{label:<label_width$} {count_label:>15}       % {cost_label:>15}       %{comment}",
            label_width = self.label_width,
        ));
    }

    pub fn add_count_cost_perc2(&mut self, label: &str, count: u64, cost: u64, comment: &str) {
        self.output += &format!(
            "{}{:<label_width$} {:>15} {:6.2}% {:>15} {:6.2}%{comment}\n",
            self.identation,
            label,
            self.format_number(count),
            count as f64 / self.step_divisor,
            self.format_number(cost),
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
            self.format_number(count),
            perc,
            self.format_number(cost),
            cost as f64 / self.cost_divisor,
            label_width = self.label_width,
        );
    }
    pub fn add_profile_tag(
        &mut self,
        label: &str,
        value: u64,
        calls: usize,
        min: u64,
        max: u64,
        total: f64,
    ) {
        self.output += &format!(
            "{}{:>15} {:6.2}% {:>10} {:>15} {:>15} {:>15} {label}\n",
            self.identation,
            self.format_number(value),
            value as f64 / total,
            self.format_number(calls as u64),
            self.format_number(value / calls as u64),
            self.format_number(min),
            self.format_number(max)
        );
    }

    pub fn add_profile_tag_steps(
        &mut self,
        label: &str,
        value: u64,
        calls: usize,
        min: u64,
        max: u64,
    ) {
        self.add_profile_tag(label, value, calls, min, max, self.step_divisor);
    }

    pub fn add_profile_tag_cost(
        &mut self,
        label: &str,
        value: u64,
        calls: usize,
        min: u64,
        max: u64,
    ) {
        self.add_profile_tag(label, value, calls, min, max, self.cost_divisor);
    }

    pub fn add_separator(&mut self) {
        self.output += &format!(
            "{}------------------------------------------------------------\n",
            self.identation
        );
    }
    pub fn add_separator_from(&mut self, from: usize) {
        let width = 48 - from;
        self.output += &format!("{}{: <from$}{:-<width$}\n", self.identation, "", "");
    }

    pub fn add_separator_width(&mut self, width: usize) {
        self.output += &format!("{}{:-<width$}\n", self.identation, "");
    }

    pub fn progress_hres_bar(&self, percentage: f64, width: usize) -> String {
        // Unicode block characters for smooth resolution
        const BLOCKS: [char; 9] = [
            '∙', // 0/8 (empty)
            '▏', // 1/8
            '▎', // 2/8 (1/4)
            '▍', // 3/8
            '▌', // 4/8 (1/2)
            '▋', // 5/8
            '▊', // 6/8 (3/4)
            '▉', // 7/8
            '█', // 8/8 (full)
        ];

        // Calculate exact fill (with 8x resolution per character)
        let exact_fill = (percentage / 100.0) * (width as f64) * 8.0;
        let full_blocks = (exact_fill / 8.0).floor() as usize;
        let partial_index = ((exact_fill % 8.0).round() as usize).min(8);

        let mut result = String::with_capacity(width);

        // Add full blocks
        for _ in 0..full_blocks.min(width) {
            result.push(BLOCKS[8]);
        }

        // Add partial block if there's space and it's not already full
        if full_blocks < width && partial_index > 0 {
            result.push(BLOCKS[partial_index]);

            // Fill remaining with empty blocks
            for _ in (full_blocks + 1)..width {
                result.push(BLOCKS[0]);
            }
        } else if full_blocks < width {
            // Fill remaining with empty blocks
            for _ in full_blocks..width {
                result.push(BLOCKS[0]);
            }
        }

        result
    }

    /// Generates a simple progress bar using full (█) and empty (░) block characters.
    pub fn progress_bar(&self, percentage: f64, width: usize) -> String {
        let filled = ((percentage / 100.0) * width as f64).round() as usize;
        let filled = filled.min(width);

        let mut result = String::with_capacity(width);
        for _ in 0..filled {
            result.push('█');
        }
        for _ in filled..width {
            result.push('░');
        }
        result
    }

    /// SDK Report - Header
    pub fn sdk_report_header(&mut self, title: &str) {
        let iwidth = self.sdk_width;
        let ibar = "═".repeat(iwidth - 2);
        self.output += &format!(
            "\n╔{ibar}╗\n\
             ║  ◆ {title: <w1$}  ║\n\
             ╠{ibar}╣\n",
            w1 = iwidth - 8
        );
    }

    pub fn sdk_report_footer(&mut self) {
        let iwidth = self.sdk_width;
        let ibar = "═".repeat(iwidth - 2);
        self.output += &format!("╚{ibar}╝\n");
    }

    pub fn sdk_report_dual_header(&mut self, title: &str, title2: &str) {
        // cost+frops:15 + frops:15 + percentage:6 + margins/separators:(3+5+1+1+3) = 49
        let width1 = self.sdk_width - 49;
        let iwidth = self.sdk_width;
        let ibar = "═".repeat(iwidth - 2);
        self.output += &format!(
            "\n╔{ibar}╗\n\
             ║  ◆ {title: <w1$}  ║  ◆ {title2: <w2$}  ║\n\
             ╠{ibar}╣\n",
            w1 = width1 - 2,
            w2 = iwidth - width1 - 13
        );
    }

    /// SDK Report - Header
    pub fn sdk_report_summary_line(&mut self, label: &str, cost: u64) {
        let lw = (self.sdk_width - 6) >> 1;
        let rw = self.sdk_width - lw - 6;
        self.output += &format!("║  {label:<lw$}{:>rw$}  ║\n", self.format_number(cost));
    }

    pub fn sdk_report_summary_data_line(&mut self, label: &str, data: &str) {
        let rw = self.sdk_width - label.len() - 6;
        self.output += &format!("║  {label}{:>rw$}  ║\n", data);
    }

    pub fn sdk_cost_distribution_title(&mut self) {
        // label:12 + cost:15 + lines/margins: 9 + percentage: 6 = 42
        let bar_width = self.sdk_width - 42;
        self.output +=
            &format!("║  {:<12} {:<bar_width$} {:>15} {:>6}  ║\n", "CATEGORY", "", "COST", "%");
    }

    pub fn sdk_cost_frops_title(&mut self) {
        // cost+frops:15 + frops:15 + percentage:6 + margins/separators:(3+5+1+1+3) = 49
        // cost:15 + percentage:6 + margins/separators:3 + 49 = 73
        let bar_width = self.sdk_width - 73 - self.label_width;
        self.output += &format!(
            "║  {:<lw$} {:<bar_width$} {:>15} {:>6}  ║  {:>15} {:>15} {:>6}  ║\n",
            "OPCODE",
            "",
            "COST",
            "%",
            "OPS + FROPS",
            "FROPS",
            "%",
            lw = self.label_width
        );
    }

    pub fn sdk_cost_distribution_separator(&mut self) {
        let bar_width = self.sdk_width - 6;
        self.output += &format!("║  {:┄<bar_width$}  ║\n", "");
    }

    pub fn sdk_cost_frops_separator(&mut self) {
        // cost+frops:15 + frops:15 + percentage:6 + margins/separators:(3+5+1+1+3) = 49
        let width1 = self.sdk_width - 49;
        self.output += &format!("║  {:┄<width1$}  ║  {:┄<38}  ║\n", "", "");
    }

    pub fn sdk_cost_distribution_line(&mut self, label: &str, cost: u64) {
        // label:12 + cost:15 + lines/margins: 9 + percentage: 6 = 42
        let bar_width = self.sdk_width - 42;
        let percentage = cost as f64 / self.cost_divisor;
        self.output += &format!(
            "║  {label:<12} {} {:>15} {percentage:>5.1}%  ║\n",
            self.progress_bar(percentage, bar_width),
            self.format_number(cost),
        );
    }

    pub fn sdk_top_cost_line_label_width(&mut self) -> usize {
        // cost:15 + lines/margins: 9 + percentage: 6 = 30
        let bar_label_width = self.sdk_width - 30;
        let bar_width = (bar_label_width / 4).min(20);
        bar_label_width - bar_width
    }

    pub fn sdk_top_cost_line(&mut self, label: &str, cost: u64) {
        // cost:15 + lines/margins: 9 + percentage: 6 = 30
        let bar_label_width = self.sdk_width - 30;
        let bar_width = (bar_label_width / 4).min(20);
        let label_width = bar_label_width - bar_width;
        let percentage = cost as f64 / self.cost_divisor;
        self.output += &format!(
            "║  {label:<label_width$} {} {:>15} {percentage:>5.1}%  ║\n",
            self.progress_bar(percentage, bar_width),
            self.format_number(cost),
        );
    }

    pub fn sdk_tag_cost_line(&mut self, label: &str, cost: u64, label_width: usize) {
        self.sdk_tag_line(label, cost, label_width, self.cost_divisor);
    }

    pub fn sdk_tag_step_line(&mut self, label: &str, cost: u64, label_width: usize) {
        self.sdk_tag_line(label, cost, label_width, self.step_divisor);
    }

    pub fn sdk_tag_line(&mut self, label: &str, cost: u64, label_width: usize, total_divisor: f64) {
        // cost:15 + lines/margins: 9 + percentage: 6 = 30
        let bar_width = self.sdk_width - 30 - label_width;
        let percentage = cost as f64 / total_divisor;
        self.output += &format!(
            "║  {label:<label_width$} {} {:>15} {percentage:>5.1}%  ║\n",
            self.progress_bar(percentage, bar_width),
            self.format_number(cost),
        );
    }

    pub fn sdk_cost_frops_line(&mut self, label: &str, cost: u64, frops_cost: Option<u64>) {
        // cost+frops:15 + frops:15 + percentage:6 + margins/separators:(3+5+1+1+3) = 49
        // cost:15 + percentage:6 + margins/separators:3 = 24
        let width1 = self.sdk_width - 49;
        let bar_width = width1 - self.label_width - 24;
        let percentage = cost as f64 / self.cost_divisor;
        if let Some(frops_cost) = frops_cost {
            let perc_frops = (frops_cost as f64 / (cost + frops_cost) as f64) * 100.0;
            self.output += &format!(
            "║  {label:<label_width$} {} {:>15} {percentage:>5.1}%  ║  {:>15} {:>15} {perc_frops:>5.1}%  ║\n",
            self.progress_bar(percentage, bar_width),
            self.format_number(cost),
            self.format_number(cost + frops_cost),
            self.format_number(frops_cost),
            label_width = self.label_width
        );
        } else {
            self.output += &format!(
                "║  {label:<label_width$} {} {:>15} {percentage:>5.1}%  ║  {: >38}  ║\n",
                self.progress_bar(percentage, bar_width),
                self.format_number(cost),
                "",
                label_width = self.label_width
            );
        }
    }

    pub fn sdk_cost_frops_total_line(&mut self, label: &str, cost: u64, frops: u64) {
        // cost+frops:15 + frops:15 + percentage:6 + margins/separators:(3+5+1+1+3) = 49
        // cost:15 + percentage:6 + margins/separators:3 = 24
        let width1 = self.sdk_width - 49;
        let bar_width = width1 - self.label_width - 24;
        let percentage = cost as f64 / self.cost_divisor;
        let perc_frops = (frops as f64 / (cost + frops) as f64) * 100.0;
        self.output += &format!(
            "║  {label:<label_width$} {:>15} {percentage:>5.1}%  ║  {:>15} {:>15} {perc_frops:>5.1}%  ║\n",
            self.format_number(cost),
            self.format_number(cost + frops),
            self.format_number(frops),
            label_width = self.label_width + bar_width + 1
        );
    }

    pub fn sdk_cost_distribution_total_line(&mut self, label: &str, cost: u64) {
        // cost:16 + lines/margins: 8 + percentage: 6 = 30
        let lw = self.sdk_width - 30;
        let percentage = cost as f64 / self.cost_divisor;
        self.output +=
            &format!("║  {label:<lw$} {:>16} {percentage:>5.1}%  ║\n", self.format_number(cost),);
    }
    /*
        /// SDK Report - Summary line with total cost
        pub fn sdk_report_summary_title(&mut self, total_cost: u64) {
            self.output += &format!(
                "║  SUMMARY                                        {:>15}   total  ║\n\
                 ║  ──────────────────────────────────────────────────────────────────────  ║\n",
                self.format_number(total_cost)
            );
        }

        /// SDK Report - Individual metric line with progress bar
        pub fn sdk_report_summary_line(
            &mut self,
            label: &str,
            cost: u64,
            percentage: f64,
            bar_width: usize,
        ) {
            self.output += &format!(
                "║  {:<12}  {}   {:>15}   {:>5.1}%  ║\n",
                label,
                self.progress_bar(percentage, bar_width),
                self.format_number(cost),
                percentage
            );
        }

        /// SDK Report - RAM usage line
        pub fn sdk_report_ram(&mut self, ram_used: u64, ram_size: u64) {
            let ram_info = if ram_size > 0 {
                let ram_used_mb = ram_used as f64 / (1024.0 * 1024.0);
                let ram_size_mb = ram_size as f64 / (1024.0 * 1024.0);
                format!("{:.1} MB / {:.1} MB", ram_used_mb, ram_size_mb)
            } else {
                "N/A".to_string()
            };

            self.output +=
                &format!("║                                               RAM   {:>19}  ║\n", ram_info);
        }
    */
}
