unit u;
interface
type
  section = record
    sectname : longint;
  end;
procedure run(var section : section);
implementation
procedure run(var section : section);
begin
  section.sectname := 7;
end;
end.
