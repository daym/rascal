unit u;
interface
type
  tbuf = array[0..127] of char;
procedure store(index : cardinal; name : pchar; isdata : longbool);
procedure demo;
implementation
var
  cstring : tbuf;
  ordinal : word;
  isdata : longbool;
procedure store(index : cardinal; name : pchar; isdata : longbool);
begin
end;
procedure demo;
begin
  store(succ(ordinal), cstring, isdata);
end;
end.
