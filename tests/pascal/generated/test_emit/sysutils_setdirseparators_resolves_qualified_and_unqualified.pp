unit u;
interface
uses sysutils;
procedure demo;
implementation
procedure demo;
var s : string; p : pchar;
begin
  s := SetDirSeparators(s);
  s := SysUtils.SetDirSeparators(s);
  s := SysUtils.StrPas(p);
end;
end.
