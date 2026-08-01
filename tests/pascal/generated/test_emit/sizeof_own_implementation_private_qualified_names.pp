unit u;
interface
procedure run;
implementation
type
  foo = record
    a : longint;
    b : longint;
    c : longint;
  end;
var
  bar : word;
procedure run;
begin
  writeln(sizeof(u.foo));
  writeln(sizeof(u.bar));
end;
end.
