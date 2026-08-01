unit u;
interface
type
  titem = class
    value : longint;
  end;
procedure callback(arg : pointer);
implementation
procedure callback(arg : pointer);
var
  item : titem absolute arg;
begin
  writeln(item.value);
end;
end.
