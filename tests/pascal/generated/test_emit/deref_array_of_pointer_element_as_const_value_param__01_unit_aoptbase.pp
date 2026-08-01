unit aoptbase;
interface
uses cpubase, aoptcpub, aasmtai;
function reginop(reg : tregister; const op : toper) : boolean;
procedure run(p1 : tai; count : aword);
implementation
function reginop(reg : tregister; const op : toper) : boolean;
begin
  reginop := reg = op.reg;
end;
procedure run(p1 : tai; count : aword);
var
  tmpresult : boolean;
begin
  tmpresult := reginop(0, pinstr(p1)^.oper[count]^);
end;
end.
