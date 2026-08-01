unit u;
interface
type tbig = record value : qword; end;
operator := (const n : qword) : tbig;
operator := (const n : int64) : tbig;
function pick(a,b : qword) : shortstring; overload;
function pick(a,b : int64) : shortstring; overload;
function pick(a,b : longint) : shortstring; overload;
function pick(const a,b : tbig) : shortstring; overload;
procedure run(c : cardinal);
implementation
operator := (const n : qword) : tbig; begin result.value := n; end;
operator := (const n : int64) : tbig; begin result.value := qword(n); end;
function pick(a,b : qword) : shortstring; begin pick := ''; end;
function pick(a,b : int64) : shortstring; begin pick := ''; end;
function pick(a,b : longint) : shortstring; begin pick := ''; end;
function pick(const a,b : tbig) : shortstring; begin pick := ''; end;
procedure run(c : cardinal);
var s : shortstring;
begin
  s := pick(c, 1);
  s := pick(c, c);
end;
end.
