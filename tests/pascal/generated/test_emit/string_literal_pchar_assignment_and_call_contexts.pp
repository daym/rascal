unit u;
interface
procedure take(p: pchar);
procedure demo;
implementation
procedure take(p: pchar);
begin
end;
procedure demo;
var p: pchar;
begin
  p := 'abc';
  take('def');
end;
end.
