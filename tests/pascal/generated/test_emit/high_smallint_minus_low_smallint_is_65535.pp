unit u;
interface
const smallint_range = high(smallint) - low(smallint);
function smallint_range_is_65535 : boolean;
function runtime_smallint_range : int64;
implementation
function smallint_range_is_65535 : boolean;
begin
  smallint_range_is_65535 := smallint_range = 65535;
end;
function runtime_smallint_range : int64;
begin
  runtime_smallint_range := high(smallint) - low(smallint);
end;
end.
