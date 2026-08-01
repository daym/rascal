unit u;
interface
function pick(c : char) : longint; overload;
function pick(const s : string) : longint; overload;
function pick(const s : ansistring) : longint; overload;
procedure demo(c : char; s : string; a : ansistring; var i : longint);
implementation
function pick(c : char) : longint; begin pick := 1; end;
function pick(const s : string) : longint; begin pick := 2; end;
function pick(const s : ansistring) : longint; begin pick := 3; end;
procedure demo(c : char; s : string; a : ansistring; var i : longint);
begin
  i := pick(UpCase(c));
  i := pick(UpCase(s));
  i := pick(UpCase(a));
  i := pick(LowerCase(c));
  i := pick(LowerCase(s));
  i := pick(LowerCase(a));
end;
end.
