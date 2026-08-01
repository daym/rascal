unit u;
interface
type
  tprocdef = class
    function flag(x : integer) : boolean;
  end;
  tprocsym = class
    function first_procdef : tprocdef;
  end;
function ok(ps : tprocsym) : boolean;
implementation
function tprocdef.flag(x : integer) : boolean;
begin
  flag := false;
end;
function tprocsym.first_procdef : tprocdef;
begin
  first_procdef := nil;
end;
function ok(ps : tprocsym) : boolean;
begin
  ok := ps.first_procdef.flag(1);
end;
end.
