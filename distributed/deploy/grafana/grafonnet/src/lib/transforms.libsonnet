// Grafana table transformations.
// Convention: filterByValue first (drop rows), organize second (rename/index/exclude).

{
  filter_equal(field, value):: {
    id: 'filterByValue',
    options: {
      type: 'include',
      match: 'all',
      filters: [{ fieldName: field, config: { id: 'equal', options: { value: value } } }],
    },
  },

  filter_not_equal_all(field, values):: {
    id: 'filterByValue',
    options: {
      type: 'include',
      match: 'all',
      filters: [
        { fieldName: field, config: { id: 'notEqual', options: { value: v } } }
        for v in values
      ],
    },
  },

  filter_equal_any(field, values):: {
    id: 'filterByValue',
    options: {
      type: 'include',
      match: 'any',
      filters: [
        { fieldName: field, config: { id: 'equal', options: { value: v } } }
        for v in values
      ],
    },
  },

  organize(index_by_name={}, rename_by_name={}, exclude_by_name={}):: {
    id: 'organize',
    options: {
      indexByName: index_by_name,
      renameByName: rename_by_name,
      excludeByName: exclude_by_name,
    },
  },

  partition_by_values(fields, keep_fields=false, as_labels=false):: {
    id: 'partitionByValues',
    options: {
      fields: fields,
      keepFields: keep_fields,
      naming: { asLabels: as_labels },
    },
  },

  histogram(bucket_count=24, bucket_size=null, combine=true):: {
    id: 'histogram',
    options: {
      bucketCount: bucket_count,
      bucketOffset: 0,
      combine: combine,
    } + (if bucket_size != null then { bucketSize: bucket_size } else {}),
  },

  filter_non_terminal_jobs(field='State'):: self.filter_not_equal_all(
    field, ['Completed', 'Failed', 'Cancelled']
  ),
}
