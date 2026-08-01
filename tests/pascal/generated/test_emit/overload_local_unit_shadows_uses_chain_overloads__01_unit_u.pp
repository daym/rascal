unit u;
interface
uses cutils;
function tostr(i : longint) : shortstring;
procedure run;
implementation
function tostr(i : longint) : shortstring; begin tostr := ''; end;
procedure run;
var x : longint;
    s : shortstring;
begin
  s := tostr(x);
end;
end.
