unit u;
interface
procedure run;
implementation
procedure run;
type
  foo = record
    x : longint;
  end;
begin
  writeln(sizeof(u.foo));
end;
end.
