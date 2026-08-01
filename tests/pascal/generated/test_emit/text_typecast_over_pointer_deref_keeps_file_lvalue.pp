unit u;
interface
procedure dump(arg : pointer);
implementation
procedure dump(arg : pointer);
begin
  write(text(arg^), 'x');
  writeln(text(arg^));
end;
end.
