unit u;
interface
uses types;
procedure fetch;
implementation
procedure fetch;
var
  r : twordrec;
  bits : dword;
begin
  r.value := 1;
  bits := dword(r);
  r := twordrec(ntole(dword(r)));
end;
end.
