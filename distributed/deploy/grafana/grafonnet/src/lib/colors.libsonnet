// ZisK brand palette and dashboard semantic colors.

{
  black: '#0C0C0C',
  dark_blue: '#1A1A24',
  midnight: '#2D2E3D',
  gray_dark: '#363636',
  gray: '#606060',
  gray_light: '#8E8E8A',
  silver_dark: '#B8B8B4',
  silver: '#E7E7E3',
  silver_light: '#F3F3F2',
  victorian_peak: '#007755',
  soft_green: '#0ABF83',
  spring_green: '#00FF7C',
  light_green: '#A4F6D0',
  background_green: '#EBFEF5',
  busy_bee: '#F4FF00',

  healthy: self.soft_green,
  warning: '#C7D300',
  critical: '#D95F5F',
  unknown: self.gray_light,

  ready: self.healthy,
  idle: self.gray_light,
  'setup needed': self.warning,
  connecting: '#6DE7BA',
  computing: self.healthy,
  contribution: '#005F44',
  prove: self.victorian_peak,
  aggregate: '#079B6E',
  execute: self.soft_green,
  wrap: self.healthy,
  'input stream': self.victorian_peak,
  'hint stream': '#6DE7BA',
  'error/disconnected': self.critical,

  outcome: {
    success: $.healthy,
    failure: $.critical,
    cancelled: $.warning,
  },

  flow: {
    active: $.healthy,
    pending: $.victorian_peak,
    completed: $.healthy,
    failed: $.critical,
  },
}
