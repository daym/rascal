unit u;
interface
type tbig = record value : qword; end;
function pick(a,b : qword; const r : tbig) : shortstring; overload;
function pick(a,b : int64; const r : tbig) : shortstring; overload;
function pick(a,b : longint; const r : tbig) : shortstring; overload;
procedure run(c : cardinal; const r : tbig);
implementation
function pick(a,b : qword; const r : tbig) : shortstring; begin pick := ''; end;
function pick(a,b : int64; const r : tbig) : shortstring; begin pick := ''; end;
function pick(a,b : longint; const r : tbig) : shortstring; begin pick := ''; end;
procedure run(c : cardinal; const r : tbig);
var s : shortstring;
begin
  s := pick(c, 1, r);
  s := pick(c, c, r);
end;
end.
