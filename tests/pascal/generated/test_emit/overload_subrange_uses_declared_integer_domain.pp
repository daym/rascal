unit u;
interface
type tsmall = 0..255;
function pick(i : byte) : shortstring; overload;
function pick(i : longint) : shortstring; overload;
function pick(i : cardinal) : shortstring; overload;
procedure run(s : tsmall);
implementation
function pick(i : byte) : shortstring; begin pick := ''; end;
function pick(i : longint) : shortstring; begin pick := ''; end;
function pick(i : cardinal) : shortstring; begin pick := ''; end;
procedure run(s : tsmall);
var out_s : shortstring;
begin
  out_s := pick(s);
end;
end.
