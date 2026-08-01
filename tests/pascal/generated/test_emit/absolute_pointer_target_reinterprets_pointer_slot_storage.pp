unit u;
interface
type
  titem = record
    value : longint;
  end;
procedure demo(raw : pointer);
implementation
procedure demo(raw : pointer);
var
  item : titem absolute raw;
begin
  writeln(item.value);
end;
end.
