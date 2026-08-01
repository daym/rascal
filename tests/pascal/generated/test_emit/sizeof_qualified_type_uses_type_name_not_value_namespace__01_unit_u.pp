unit u;
interface
uses macho;
procedure demo;
implementation
procedure demo;
begin
  writeln(sizeof(macho.section));
  writeln(sizeof(macho.counter));
end;
end.
