unit rax86att;
interface
uses rax86int, raatt;
procedure run(act : raatt.tasmtoken);
implementation
procedure run(act : raatt.tasmtoken);
begin
  if act in [as_comma, as_separator, as_end] then begin end;
end;
end.
