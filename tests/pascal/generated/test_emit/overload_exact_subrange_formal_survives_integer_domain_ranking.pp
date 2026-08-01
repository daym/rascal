unit u;
interface
type tsmall = 0..255;
function pick(i : tsmall) : shortstring; overload;
function pick(i : longint) : shortstring; overload;
procedure run(s : tsmall);
implementation
function pick(i : tsmall) : shortstring; begin pick := ''; end;
function pick(i : longint) : shortstring; begin pick := ''; end;
procedure run(s : tsmall);
var out_s : shortstring;
begin
  out_s := pick(s);
end;
end.
