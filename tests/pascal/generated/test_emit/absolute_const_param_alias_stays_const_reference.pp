unit u;
interface
procedure demo(const s : string);
implementation
procedure demo(const s : string);
var
  view : string absolute s;
begin
  writeln(view);
end;
end.
