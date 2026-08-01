unit u;
interface
type
  efoo = class(exception);
procedure boom;
implementation
procedure boom;
begin
  raise efoo.create('bad');
end;
end.
