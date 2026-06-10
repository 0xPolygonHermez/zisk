// Panel constructors used by dashboard section files.

local g = import 'github.com/grafana/grafonnet/gen/grafonnet-v11.2.0/main.libsonnet';
local tr = import 'lib/transforms.libsonnet';

local prom_ds = { type: 'prometheus', uid: 'prometheus' };

{
  prom_instant(expr, legend='', ref_id='A'):: {
    refId: ref_id,
    datasource: prom_ds,
    expr: std.stripChars(expr, '\n '),
    instant: true,
    range: false,
    legendFormat: legend,
  },

  prom_range(expr, legend='', ref_id='A'):: {
    refId: ref_id,
    datasource: prom_ds,
    expr: std.stripChars(expr, '\n '),
    instant: false,
    range: true,
    legendFormat: legend,
  },

  prom_heatmap(expr, legend='', ref_id='A'):: self.prom_range(expr, legend, ref_id) + {
    format: 'heatmap',
  },

  by_name_color(name, color):: {
    matcher: { id: 'byName', options: name },
    properties: [{ id: 'color', value: { mode: 'fixed', fixedColor: color } }],
  },

  by_name(name, props):: {
    matcher: { id: 'byName', options: name },
    properties: props,
  },

  prop_unit(u):: { id: 'unit', value: u },
  prop_decimals(d):: { id: 'decimals', value: d },
  prop_width(w):: { id: 'custom.width', value: w },
  prop_mappings(m):: { id: 'mappings', value: m },
  prop_cell_color_text:: { id: 'custom.cellOptions', value: { type: 'color-text' } },
  prop_cell_color_bg:: { id: 'custom.cellOptions', value: { type: 'color-background' } },
  prop_thresholds(steps, mode='absolute'):: { id: 'thresholds', value: { mode: mode, steps: steps } },

  stat(id, title, pos, target, description='', unit='none', decimals=0, thresholds=[], color_mode='value', graph_mode='none', text_mode='value', justify='center', no_value='-', mappings=[], calcs=['lastNotNull'], overrides=[], title_size=null, value_size=null)::
    g.panel.stat.new(title)
    + { id: id, gridPos: pos, datasource: target.datasource }
    + g.panel.stat.panelOptions.withDescription(description)
    + g.panel.stat.queryOptions.withTargets([target])
    + g.panel.stat.standardOptions.withUnit(unit)
    + g.panel.stat.standardOptions.withDecimals(decimals)
    + g.panel.stat.standardOptions.withNoValue(no_value)
    + g.panel.stat.standardOptions.thresholds.withMode('absolute')
    + g.panel.stat.standardOptions.thresholds.withSteps(thresholds)
    + g.panel.stat.standardOptions.withMappings(mappings)
    + g.panel.stat.standardOptions.withOverrides(overrides)
    + g.panel.stat.options.withColorMode(color_mode)
    + g.panel.stat.options.withGraphMode(graph_mode)
    + g.panel.stat.options.withTextMode(text_mode)
    + g.panel.stat.options.withJustifyMode(justify)
    + g.panel.stat.options.reduceOptions.withValues(false)
    + g.panel.stat.options.reduceOptions.withCalcs(calcs)
    + (if title_size != null then g.panel.stat.options.text.withTitleSize(title_size) else {})
    + (if value_size != null then g.panel.stat.options.text.withValueSize(value_size) else {}),

  // Keep decimals=null for string-valued fields; Grafana may null strings when decimal formatting is set.
  json_stat(id, title, pos, target, description='', transformations=[], unit='none', decimals=null, calcs=['lastNotNull'], thresholds=[], no_value='-', mappings=[], overrides=[], color_mode='value', graph_mode='none', text_mode='value', justify='center', values=false)::
    g.panel.stat.new(title)
    + { id: id, gridPos: pos, datasource: target.datasource }
    + g.panel.stat.panelOptions.withDescription(description)
    + g.panel.stat.queryOptions.withTargets([target])
    + g.panel.stat.queryOptions.withTransformations(transformations)
    + g.panel.stat.standardOptions.withUnit(unit)
    + (if decimals != null then g.panel.stat.standardOptions.withDecimals(decimals) else {})
    + g.panel.stat.standardOptions.withNoValue(no_value)
    + g.panel.stat.standardOptions.thresholds.withMode('absolute')
    + g.panel.stat.standardOptions.thresholds.withSteps(thresholds)
    + g.panel.stat.standardOptions.withMappings(mappings)
    + g.panel.stat.standardOptions.withOverrides(overrides)
    + g.panel.stat.options.withColorMode(color_mode)
    + g.panel.stat.options.withGraphMode(graph_mode)
    + g.panel.stat.options.withTextMode(text_mode)
    + g.panel.stat.options.withJustifyMode(justify)
    + g.panel.stat.options.reduceOptions.withValues(values)
    + g.panel.stat.options.reduceOptions.withCalcs(calcs),

  timeseries(id, title, pos, targets, description='', unit='none', decimals=2, thresholds=[], thresholds_style=null, min=null, line_width=1, overrides=[], legend_mode='table', legend_show=true, fill_opacity=0, stacking_mode='none')::
    g.panel.timeSeries.new(title)
    + { id: id, gridPos: pos, datasource: targets[0].datasource }
    + g.panel.timeSeries.panelOptions.withDescription(description)
    + g.panel.timeSeries.queryOptions.withTargets(targets)
    + g.panel.timeSeries.standardOptions.withUnit(unit)
    + g.panel.timeSeries.standardOptions.withDecimals(decimals)
    + g.panel.timeSeries.standardOptions.withNoValue('-')
    + g.panel.timeSeries.standardOptions.thresholds.withMode('absolute')
    + g.panel.timeSeries.standardOptions.thresholds.withSteps(thresholds)
    + g.panel.timeSeries.standardOptions.withOverrides(overrides)
    + g.panel.timeSeries.fieldConfig.defaults.custom.withLineWidth(line_width)
    + g.panel.timeSeries.fieldConfig.defaults.custom.withFillOpacity(fill_opacity)
    + g.panel.timeSeries.fieldConfig.defaults.custom.withStacking({ mode: stacking_mode, group: 'A' })
    + (if min != null then g.panel.timeSeries.standardOptions.withMin(min) else {})
    + (if thresholds_style != null then g.panel.timeSeries.fieldConfig.defaults.custom.thresholdsStyle.withMode(thresholds_style) else {})
    + {
      options+: {
        legend: {
          showLegend: legend_show,
          displayMode: legend_mode,
          placement: 'bottom',
          calcs: ['lastNotNull', 'max'],
        },
        tooltip: { mode: 'multi', sort: 'none' },
      },
    },

  bar_chart(id, title, pos, targets, description='', x_field='Job', orientation='horizontal', stacking='normal', unit='ms', decimals=0, overrides=[], transformations=[], show_value='always', legend_mode='list', x_tick_label_spacing=0, x_tick_label_rotation=0, gradient_mode='none', fill_opacity=80, line_width=0, bar_radius=0, bar_width=0.97, group_width=0.7)::
    g.panel.barChart.new(title)
    + { id: id, gridPos: pos, datasource: targets[0].datasource }
    + g.panel.barChart.panelOptions.withDescription(description)
    + g.panel.barChart.queryOptions.withTargets(targets)
    + g.panel.barChart.queryOptions.withTransformations(transformations)
    + g.panel.barChart.standardOptions.withUnit(unit)
    + g.panel.barChart.standardOptions.withDecimals(decimals)
    + g.panel.barChart.standardOptions.withNoValue('-')
    + g.panel.barChart.standardOptions.withOverrides(overrides)
    + g.panel.barChart.options.withOrientation(orientation)
    + g.panel.barChart.options.withStacking(stacking)
    + g.panel.barChart.options.withXField(x_field)
    + g.panel.barChart.options.withShowValue(show_value)
    + g.panel.barChart.options.withXTickLabelSpacing(x_tick_label_spacing)
    + g.panel.barChart.options.withXTickLabelRotation(x_tick_label_rotation)
    + g.panel.barChart.options.withBarRadius(bar_radius)
    + g.panel.barChart.options.withBarWidth(bar_width)
    + g.panel.barChart.options.withGroupWidth(group_width)
    + g.panel.barChart.options.legend.withDisplayMode(legend_mode)
    + g.panel.barChart.options.legend.withPlacement('bottom')
    + g.panel.barChart.fieldConfig.defaults.custom.withGradientMode(gradient_mode)
    + g.panel.barChart.fieldConfig.defaults.custom.withFillOpacity(fill_opacity)
    + g.panel.barChart.fieldConfig.defaults.custom.withLineWidth(line_width),

  heatmap(id, title, pos, targets, description='', unit='s', decimals=2, y_axis_label='Duration bucket', legend=true)::
    g.panel.heatmap.new(title)
    + { id: id, gridPos: pos, datasource: targets[0].datasource }
    + g.panel.heatmap.panelOptions.withDescription(description)
    + g.panel.heatmap.queryOptions.withTargets(targets)
    + g.panel.heatmap.standardOptions.withUnit(unit)
    + g.panel.heatmap.standardOptions.withDecimals(decimals)
    + g.panel.heatmap.standardOptions.withNoValue('-')
    + g.panel.heatmap.options.withCalculate(false)
    + g.panel.heatmap.options.withCellGap(1)
    + g.panel.heatmap.options.withShowValue('never')
    + g.panel.heatmap.options.withColor({
      mode: 'scheme',
      scheme: 'Spectral',
      steps: 64,
      reverse: false,
      exponent: 0.5,
    })
    + g.panel.heatmap.options.withLegend({ show: legend })
    + g.panel.heatmap.options.withTooltip({ mode: 'single', showColorScale: true, yHistogram: true })
    + g.panel.heatmap.options.withYAxis({
      axisLabel: y_axis_label,
      axisPlacement: 'left',
      decimals: decimals,
      reverse: false,
      unit: unit,
    }),

  xy_chart(id, title, pos, targets, description='', x_field='', y_field='', x_axis_label='X', y_axis_label='Y', x_unit='short', y_unit='ms', x_decimals=0, y_decimals=0, point_size=8, line_width=1, show='points', overrides=[], transformations=[])::
    g.panel.xyChart.new(title)
    + { id: id, gridPos: pos, datasource: targets[0].datasource, type: 'xychart' }
    + g.panel.xyChart.panelOptions.withDescription(description)
    + g.panel.xyChart.queryOptions.withTargets(targets)
    + g.panel.xyChart.queryOptions.withTransformations(transformations)
    + g.panel.xyChart.standardOptions.withOverrides(overrides)
    + g.panel.xyChart.fieldConfig.defaults.custom.withShow(show)
    + g.panel.xyChart.fieldConfig.defaults.custom.withAxisLabel(y_axis_label)
    + g.panel.xyChart.fieldConfig.defaults.custom.withLineWidth(line_width)
    + g.panel.xyChart.options.withSeriesMapping('manual')
    + g.panel.xyChart.options.withSeries([{ frame: 0, x: x_field, y: y_field, name: y_axis_label }])
    + g.panel.xyChart.options.withTooltip({ mode: 'single', sort: 'none' })
    + g.panel.xyChart.options.legend.withShowLegend(true)
    + g.panel.xyChart.options.legend.withDisplayMode('list')
    + g.panel.xyChart.options.legend.withPlacement('bottom')
    + {
      fieldConfig+: {
        defaults+: {
          unit: y_unit,
          decimals: y_decimals,
          custom+: {
            pointSize: { fixed: point_size },
            axisGridShow: true,
          },
        },
        overrides+: [{
          matcher: { id: 'byName', options: x_field },
          properties: [
            { id: 'unit', value: x_unit },
            { id: 'decimals', value: x_decimals },
            { id: 'custom.axisLabel', value: x_axis_label },
          ],
        }],
      },
    },

  trend_chart(id, title, pos, targets, description='', x_field='', x_axis_label='X', y_axis_label='Y', x_unit='short', y_unit='s', x_decimals=0, y_decimals=1, point_size=6, overrides=[], transformations=[])::
    g.panel.trend.new(title)
    + { id: id, gridPos: pos, datasource: targets[0].datasource }
    + g.panel.trend.panelOptions.withDescription(description)
    + g.panel.trend.queryOptions.withTargets(targets)
    + g.panel.trend.queryOptions.withTransformations(transformations)
    + g.panel.trend.standardOptions.withUnit(y_unit)
    + g.panel.trend.standardOptions.withDecimals(y_decimals)
    + g.panel.trend.standardOptions.withNoValue('-')
    + g.panel.trend.standardOptions.withOverrides(overrides)
    + g.panel.trend.fieldConfig.defaults.custom.withDrawStyle('points')
    + g.panel.trend.fieldConfig.defaults.custom.withShowPoints('always')
    + g.panel.trend.fieldConfig.defaults.custom.withPointSize(point_size)
    + g.panel.trend.fieldConfig.defaults.custom.withLineWidth(0)
    + g.panel.trend.fieldConfig.defaults.custom.withAxisLabel(y_axis_label)
    + g.panel.trend.options.withXField(x_field)
    + g.panel.trend.options.legend.withShowLegend(true)
    + g.panel.trend.options.legend.withDisplayMode('list')
    + g.panel.trend.options.legend.withPlacement('bottom')
    + g.panel.trend.options.tooltip.withMode('single')
    + g.panel.trend.options.tooltip.withSort('none')
    + {
      fieldConfig+: {
        defaults+: {
          custom+: {
            axisGridShow: true,
          },
        },
        overrides+: [{
          matcher: { id: 'byName', options: x_field },
          properties: [
            { id: 'unit', value: x_unit },
            { id: 'decimals', value: x_decimals },
            { id: 'custom.axisLabel', value: x_axis_label },
          ],
        }],
      },
    },

  // Raw-value histogram for persisted history rows; Prometheus buckets use heatmaps or quantiles.
  histogram(id, title, pos, targets, description='', unit='ms', decimals=0, bucket_count=24, bucket_size=null, combine=true, overrides=[], transformations=[], legend=true, axis_label='Duration')::
    g.panel.histogram.new(title)
    + { id: id, gridPos: pos, datasource: targets[0].datasource }
    + g.panel.histogram.panelOptions.withDescription(description)
    + g.panel.histogram.queryOptions.withTargets(targets)
    + g.panel.histogram.queryOptions.withTransformations(
      transformations + [tr.histogram(bucket_count=bucket_count, bucket_size=bucket_size, combine=combine)]
    )
    + g.panel.histogram.standardOptions.withUnit(unit)
    + g.panel.histogram.standardOptions.withDecimals(decimals)
    + g.panel.histogram.standardOptions.withNoValue('-')
    + g.panel.histogram.standardOptions.withOverrides(overrides)
    + g.panel.histogram.fieldConfig.defaults.custom.withAxisLabel(axis_label)
    + g.panel.histogram.fieldConfig.defaults.custom.withFillOpacity(80)
    + g.panel.histogram.fieldConfig.defaults.custom.withLineWidth(1)
    // bucket_count remains set even when bucket_size is used so validator rules stay stable.
    + g.panel.histogram.options.withBucketCount(bucket_count)
    + (if bucket_size != null then g.panel.histogram.options.withBucketSize(bucket_size) else {})
    + g.panel.histogram.options.withBucketOffset(0)
    + g.panel.histogram.options.withCombine(combine)
    + g.panel.histogram.options.legend.withShowLegend(legend)
    + g.panel.histogram.options.legend.withDisplayMode('list')
    + g.panel.histogram.options.legend.withPlacement('bottom')
    + g.panel.histogram.options.tooltip.withMode('single')
    + g.panel.histogram.options.tooltip.withSort('none'),

  bar_gauge(id, title, pos, targets, description='', unit='none', decimals=0, thresholds=[], min=0, overrides=[], name_placement='left', title_size=null, value_size=null, min_viz_height=null, max_viz_height=null, values=false)::
    g.panel.barGauge.new(title)
    + { id: id, gridPos: pos, datasource: targets[0].datasource }
    + g.panel.barGauge.panelOptions.withDescription(description)
    + g.panel.barGauge.queryOptions.withTargets(targets)
    + g.panel.barGauge.standardOptions.withUnit(unit)
    + g.panel.barGauge.standardOptions.withDecimals(decimals)
    + g.panel.barGauge.standardOptions.withNoValue('-')
    + g.panel.barGauge.standardOptions.withMin(min)
    + g.panel.barGauge.standardOptions.thresholds.withMode('absolute')
    + g.panel.barGauge.standardOptions.thresholds.withSteps(thresholds)
    + g.panel.barGauge.standardOptions.withOverrides(overrides)
    + g.panel.barGauge.options.withDisplayMode('gradient')
    + g.panel.barGauge.options.withOrientation('horizontal')
    + g.panel.barGauge.options.withShowUnfilled(true)
    + g.panel.barGauge.options.withValueMode('color')
    + g.panel.barGauge.options.withNamePlacement(name_placement)
    + g.panel.barGauge.options.reduceOptions.withValues(values)
    + g.panel.barGauge.options.reduceOptions.withCalcs(['lastNotNull'])
    + (if title_size != null then g.panel.barGauge.options.text.withTitleSize(title_size) else {})
    + (if value_size != null then g.panel.barGauge.options.text.withValueSize(value_size) else {})
    + (if min_viz_height != null then g.panel.barGauge.options.withMinVizHeight(min_viz_height) else {})
    + (if max_viz_height != null then g.panel.barGauge.options.withMaxVizHeight(max_viz_height) else {}),

  table(id, title, pos, targets, description='', transformations=[], mappings=[], overrides=[], no_value='-', cell_color_bg=false, show_header=true, cell_height='sm')::
    g.panel.table.new(title)
    + { id: id, gridPos: pos, datasource: targets[0].datasource }
    + g.panel.table.panelOptions.withDescription(description)
    + g.panel.table.queryOptions.withTargets(targets)
    + g.panel.table.queryOptions.withTransformations(transformations)
    + g.panel.table.standardOptions.withNoValue(no_value)
    + g.panel.table.standardOptions.withMappings(mappings)
    + g.panel.table.standardOptions.withOverrides(overrides)
    + {
      options+: {
        showHeader: show_header,
        cellHeight: cell_height,
        footer: { show: false },
      },
    }
    + (
      if cell_color_bg then {
        fieldConfig+: {
          defaults+: {
            custom+: {
              cellOptions: { type: 'color-background' },
            },
          },
        },
      } else {}
    ),

  state_timeline(id, title, pos, targets, description='', mappings=[], transformations=[], overrides=[], merge_values=false, show_value='never', align_value='center', fill_opacity=85, line_width=0, row_height=0.9, legend=true, no_value='-')::
    g.panel.stateTimeline.new(title)
    + { id: id, gridPos: pos, datasource: targets[0].datasource }
    + g.panel.stateTimeline.panelOptions.withDescription(description)
    + g.panel.stateTimeline.queryOptions.withTargets(targets)
    + g.panel.stateTimeline.queryOptions.withTransformations(transformations)
    + g.panel.stateTimeline.standardOptions.withNoValue(no_value)
    + g.panel.stateTimeline.standardOptions.withMappings(mappings)
    + g.panel.stateTimeline.standardOptions.withOverrides(overrides)
    + g.panel.stateTimeline.fieldConfig.defaults.custom.withLineWidth(line_width)
    + g.panel.stateTimeline.fieldConfig.defaults.custom.withFillOpacity(fill_opacity)
    + g.panel.stateTimeline.options.withMergeValues(merge_values)
    + g.panel.stateTimeline.options.withShowValue(show_value)
    + g.panel.stateTimeline.options.withAlignValue(align_value)
    + {
      fieldConfig+: {
        defaults+: {
          custom+: {
            spanNulls: false,
          },
        },
      },
      options+: {
        rowHeight: row_height,
        tooltip: { mode: 'multi', sort: 'none' },
      } + (
        if legend then {
          legend: {
            showLegend: true,
            displayMode: 'table',
            placement: 'bottom',
            calcs: ['lastNotNull'],
          },
        } else {}
      ),
    },

  row(id, title, pos, collapsed=false, panels=[])::
    g.panel.row.new(title)
    + { id: id, gridPos: pos, collapsed: collapsed, panels: panels },
}
