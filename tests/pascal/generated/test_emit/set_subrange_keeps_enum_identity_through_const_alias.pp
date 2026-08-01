unit u;
interface
type
  ttoken = (none, plus, minus, star);
  tlevel = (addition, multiplication);
const
  lastoperator = star;
  levels : array[tlevel] of set of none..lastoperator =
    ([plus, minus], [star]);
implementation
end.
