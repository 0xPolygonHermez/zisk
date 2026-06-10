// Infinity datasource target + column builders.

{
  target(url, columns, root_selector='data', ref_id='A'):: {
    refId: ref_id,
    datasource: { type: 'yesoreyeram-infinity-datasource', uid: 'zisk-json' },
    type: 'json',
    source: 'url',
    format: 'table',
    parser: 'backend',
    url: url,
    url_options: { method: 'GET', data: '' },
    root_selector: root_selector,
    columns: columns,
  },

  column(selector, text, type='string'):: {
    selector: selector,
    text: text,
    type: type,
  },

  string(selector, text):: self.column(selector, text, 'string'),
  number(selector, text):: self.column(selector, text, 'number'),
  timestamp(selector, text):: self.column(selector, text, 'timestamp'),
}
